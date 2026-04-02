use anyhow::{anyhow, Result};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, Request,
};
use libc::{EACCES, EINVAL, EIO, ENOENT, ENOTSUP, O_ACCMODE, O_RDONLY};
use scred_redactor::metadata_cache::RiskTier;
use scred_redactor::{PatternSelector, RedactionConfig, RedactionEngine, StreamingRedactor};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::ffi::OsStr;
use std::fs;
use std::hash::{Hash, Hasher};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);
const FOPEN_DIRECT_IO: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPolicy {
    Deny,
    Passthrough,
}

#[derive(Debug, Clone)]
pub struct MountConfig {
    pub source: PathBuf,
    pub binary_policy: BinaryPolicy,
    pub max_snapshot_size: usize,
    pub lookahead_size: usize,
    pub chunk_size: usize,
    pub redact_selector: Option<PatternSelector>,
}

impl MountConfig {
    pub fn mount_options(&self) -> Vec<MountOption> {
        vec![
            MountOption::RO,
            MountOption::FSName("scred".into()),
            MountOption::AutoUnmount,
            MountOption::NoExec,
            MountOption::NoAtime,
            MountOption::DefaultPermissions,
        ]
    }
}

impl Default for MountConfig {
    fn default() -> Self {
        Self {
            source: PathBuf::new(),
            binary_policy: BinaryPolicy::Deny,
            max_snapshot_size: 8 * 1024 * 1024,
            lookahead_size: 512,
            chunk_size: 64 * 1024,
            redact_selector: None,
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    ino: u64,
    rel: PathBuf,
    kind: FileType,
}

#[derive(Debug, Clone)]
struct Handle {
    data: Arc<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SnapshotKey {
    rel: PathBuf,
    size: u64,
    mtime_sec: i64,
    mtime_nsec: i64,
}

pub struct RedactingFs {
    config: MountConfig,
    nodes: BTreeMap<u64, Node>,
    path_to_ino: HashMap<PathBuf, u64>,
    next_handle: u64,
    handles: HashMap<u64, Handle>,
    engine: Arc<RedactionEngine>,
    snapshot_cache: Mutex<HashMap<SnapshotKey, Arc<Vec<u8>>>>,
}

impl RedactingFs {
    pub fn snapshot_for_path(&self, rel: &Path) -> Result<Vec<u8>> {
        let ino = *self
            .path_to_ino
            .get(rel)
            .ok_or_else(|| anyhow!("path not indexed: {}", rel.display()))?;
        let node = self.node(ino).ok_or_else(|| anyhow!("inode not indexed: {ino}"))?;
        self.snapshot_file(node)
    }

    pub fn new(config: MountConfig) -> Result<Self> {
        if !config.source.is_dir() {
            return Err(anyhow!("source is not a directory: {}", config.source.display()));
        }

        let mut fs = Self {
            config,
            nodes: BTreeMap::new(),
            path_to_ino: HashMap::new(),
            next_handle: 1,
            handles: HashMap::new(),
            engine: Arc::new(RedactionEngine::new(RedactionConfig::default())),
            snapshot_cache: Mutex::new(HashMap::new()),
        };
        fs.build_index()?;
        Ok(fs)
    }

    fn build_index(&mut self) -> Result<()> {
        self.nodes.clear();
        self.path_to_ino.clear();
        self.insert_node(PathBuf::new(), FileType::Directory);
        self.walk_dir(PathBuf::new())?;
        Ok(())
    }

    fn walk_dir(&mut self, rel: PathBuf) -> Result<()> {
        let abs = self.abs_path(&rel)?;
        for entry in fs::read_dir(abs)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            let child_rel = rel.join(name);

            if file_type.is_symlink() {
                continue;
            } else if file_type.is_dir() {
                self.insert_node(child_rel.clone(), FileType::Directory);
                self.walk_dir(child_rel)?;
            } else if file_type.is_file() {
                if self.should_expose_file(&child_rel)? {
                    self.insert_node(child_rel, FileType::RegularFile);
                }
            }
        }
        Ok(())
    }

    fn insert_node(&mut self, rel: PathBuf, kind: FileType) -> u64 {
        let ino = stable_ino(&rel, kind);
        self.nodes.insert(ino, Node { ino, rel: rel.clone(), kind });
        self.path_to_ino.insert(rel, ino);
        ino
    }

    fn abs_path(&self, rel: &Path) -> Result<PathBuf> {
        if rel.is_absolute() {
            return Err(anyhow!("absolute path not allowed"));
        }
        for component in rel.components() {
            if matches!(component, Component::ParentDir | Component::Prefix(_)) {
                return Err(anyhow!("path traversal not allowed"));
            }
        }
        Ok(self.config.source.join(rel))
    }

    fn node(&self, ino: u64) -> Option<&Node> {
        self.nodes.get(&ino)
    }

    fn should_expose_file(&self, rel: &Path) -> Result<bool> {
        let abs = self.abs_path(rel)?;
        let data = fs::read(abs)?;
        if is_likely_text(&data) {
            return Ok(true);
        }
        Ok(matches!(self.config.binary_policy, BinaryPolicy::Passthrough))
    }

    fn file_attr(&self, node: &Node) -> Result<FileAttr> {
        let abs = self.abs_path(&node.rel)?;
        let meta = fs::symlink_metadata(&abs)?;
        let size = if node.kind == FileType::RegularFile {
            match self.snapshot_len(node) {
                Ok(n) => n as u64,
                Err(_) => meta.len(),
            }
        } else {
            0
        };
        Ok(FileAttr {
            ino: node.ino,
            size,
            blocks: meta.blocks(),
            atime: system_time(meta.atime(), meta.atime_nsec()),
            mtime: system_time(meta.mtime(), meta.mtime_nsec()),
            ctime: system_time(meta.ctime(), meta.ctime_nsec()),
            crtime: UNIX_EPOCH,
            kind: node.kind,
            perm: safe_perm(node.kind, meta.permissions().mode()),
            nlink: if node.kind == FileType::Directory { 2 } else { 1 },
            uid: meta.uid(),
            gid: meta.gid(),
            rdev: 0,
            blksize: self.config.chunk_size as u32,
            flags: 0,
        })
    }

    fn snapshot_len(&self, node: &Node) -> Result<usize> {
        Ok(self.snapshot_file_arc(node)?.len())
    }

    fn snapshot_key(&self, node: &Node) -> Result<SnapshotKey> {
        let abs = self.abs_path(&node.rel)?;
        let meta = fs::metadata(abs)?;
        Ok(SnapshotKey {
            rel: node.rel.clone(),
            size: meta.len(),
            mtime_sec: meta.mtime(),
            mtime_nsec: meta.mtime_nsec(),
        })
    }

    fn snapshot_file_arc(&self, node: &Node) -> Result<Arc<Vec<u8>>> {
        let key = self.snapshot_key(node)?;
        if let Some(existing) = self.snapshot_cache.lock().unwrap().get(&key).cloned() {
            return Ok(existing);
        }

        let abs = self.abs_path(&node.rel)?;
        let data = fs::read(&abs)?;
        if data.len() > self.config.max_snapshot_size {
            return Err(anyhow!("file too large for poc snapshot: {} bytes", data.len()));
        }

        let out = if is_likely_text(&data) {
            if let Some(selector) = &self.config.redact_selector {
                redact_bytes_with_selector(&data, selector)
            } else {
                let redactor = StreamingRedactor::new(
                    self.engine.clone(),
                    scred_redactor::StreamingConfig {
                        chunk_size: self.config.chunk_size,
                        lookahead_size: self.config.lookahead_size,
                    },
                );
                let (out, _) = redactor.redact_buffer_bytes(&data);
                out
            }
        } else {
            match self.config.binary_policy {
                BinaryPolicy::Deny => return Err(anyhow!("binary file denied")),
                BinaryPolicy::Passthrough => data,
            }
        };

        let out = Arc::new(out);
        self.snapshot_cache.lock().unwrap().insert(key, out.clone());
        Ok(out)
    }

    fn snapshot_file(&self, node: &Node) -> Result<Vec<u8>> {
        Ok((*self.snapshot_file_arc(node)?).clone())
    }
}

impl Filesystem for RedactingFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let Some(parent_node) = self.node(parent) else {
            reply.error(ENOENT);
            return;
        };
        if parent_node.kind != FileType::Directory {
            reply.error(ENOENT);
            return;
        }
        let rel = if parent_node.rel.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            parent_node.rel.join(name)
        };
        let Some(ino) = self.path_to_ino.get(&rel).copied() else {
            reply.error(ENOENT);
            return;
        };
        match self.node(ino).and_then(|n| self.file_attr(n).ok().map(|a| (n, a))) {
            Some((_node, attr)) => reply.entry(&TTL, &attr, 0),
            None => reply.error(EIO),
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        match self.node(ino).and_then(|n| self.file_attr(n).ok()) {
            Some(attr) => reply.attr(&TTL, &attr),
            None => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let Some(dir) = self.node(ino).cloned() else {
            reply.error(ENOENT);
            return;
        };
        if dir.kind != FileType::Directory {
            reply.error(ENOENT);
            return;
        }

        let mut entries: Vec<(u64, FileType, String)> = vec![
            (dir.ino, FileType::Directory, ".".into()),
            (self.path_to_ino.get(&parent_rel(&dir.rel)).copied().unwrap_or(dir.ino), FileType::Directory, "..".into()),
        ];

        for node in self.nodes.values() {
            if parent_rel(&node.rel) == dir.rel && !node.rel.as_os_str().is_empty() {
                if let Some(name) = node.rel.file_name().and_then(|s| s.to_str()) {
                    entries.push((node.ino, node.kind, name.to_string()));
                }
            }
        }
        entries.sort_by(|a, b| a.2.cmp(&b.2));

        for (i, (child_ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            let full = reply.add(child_ino, (i + 1) as i64, kind, name);
            if full {
                break;
            }
        }
        reply.ok();
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        if flags & O_ACCMODE != O_RDONLY {
            reply.error(EACCES);
            return;
        }
        let Some(node) = self.node(ino).cloned() else {
            reply.error(ENOENT);
            return;
        };
        if node.kind != FileType::RegularFile {
            reply.error(EINVAL);
            return;
        }

        match self.snapshot_file_arc(&node) {
            Ok(data) => {
                let fh = self.next_handle;
                self.next_handle += 1;
                self.handles.insert(fh, Handle { data });
                reply.opened(fh, FOPEN_DIRECT_IO);
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("binary file denied") {
                    reply.error(ENOTSUP);
                } else if msg.contains("too large") {
                    reply.error(EACCES);
                } else {
                    reply.error(EIO);
                }
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let Some(handle) = self.handles.get(&fh) else {
            reply.error(ENOENT);
            return;
        };
        if offset < 0 {
            reply.error(EINVAL);
            return;
        }
        let start = offset as usize;
        let end = start.saturating_add(size as usize).min(handle.data.len());
        if start >= handle.data.len() {
            reply.data(&[]);
            return;
        }
        reply.data(&handle.data[start..end]);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.handles.remove(&fh);
        reply.ok();
    }
}

fn stable_ino(path: &Path, kind: FileType) -> u64 {
    if path.as_os_str().is_empty() {
        return 1;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    kind.hash(&mut hasher);
    let n = hasher.finish();
    if n == 1 { 2 } else { n }
}

fn parent_rel(path: &Path) -> PathBuf {
    path.parent().map(Path::to_path_buf).unwrap_or_default()
}

fn system_time(secs: i64, nanos: i64) -> SystemTime {
    if secs < 0 || nanos < 0 {
        UNIX_EPOCH
    } else {
        UNIX_EPOCH + Duration::new(secs as u64, nanos as u32)
    }
}

fn safe_perm(kind: FileType, mode: u32) -> u16 {
    let base = (mode & 0o777) as u16;
    match kind {
        FileType::Directory => base & 0o555,
        FileType::RegularFile => base & 0o444,
        _ => 0o444,
    }
}

fn tier_from_detector_type(pattern_type: u16) -> RiskTier {
    match pattern_type {
        0..=99 => RiskTier::Critical,
        100..=199 => RiskTier::ApiKeys,
        200..=299 => RiskTier::Patterns,
        300..=399 => RiskTier::Infrastructure,
        _ => RiskTier::Services,
    }
}

fn redact_bytes_with_selector(data: &[u8], selector: &PatternSelector) -> Vec<u8> {
    let detection = scred_redactor::scred_detector::detect_all(data);
    let mut selected = Vec::with_capacity(detection.matches.len());

    for m in &detection.matches {
        let tier = tier_from_detector_type(m.pattern_type);
        let tier_name = tier.name();
        if selector.matches_pattern(tier_name, tier) {
            selected.push(m.clone());
        }
    }

    if selected.is_empty() {
        return data.to_vec();
    }

    let mut out = data.to_vec();
    scred_redactor::scred_detector::redact_in_place(&mut out, &selected);
    out
}

fn is_likely_text(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(8192)];
    if sample.is_empty() {
        return true;
    }
    let nul_count = sample.iter().filter(|b| **b == 0).count();
    if nul_count > 0 {
        return false;
    }
    std::str::from_utf8(sample).is_ok()
}
