//! تحميل ملفات مع التحقق من SHA1، واستكمال Range، وإعادة المحاولة، وتقدّم/سرعة.
use crate::{Error, Result};
use futures_util::StreamExt;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct Job {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// عدّادات مشتركة تقرأها الواجهة لعرض النسبة والسرعة.
#[derive(Debug, Default)]
pub struct Progress {
    pub total_bytes: AtomicU64,
    pub done_bytes: AtomicU64,
    pub files_total: AtomicU64,
    pub files_done: AtomicU64,
}

pub async fn sha1_file(path: &Path) -> Result<String> {
    use tokio::io::AsyncReadExt;
    let mut f = tokio::fs::File::open(path).await?;
    let mut h = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// هل الملف موجود وسليم؟ (يعيد false للتالف فيُحمَّل من جديد)
pub async fn is_valid(job: &Job) -> bool {
    let Ok(meta) = tokio::fs::metadata(&job.dest).await else { return false };
    if let Some(s) = job.size {
        if meta.len() != s {
            return false;
        }
    }
    match &job.sha1 {
        Some(expected) => sha1_file(&job.dest).await.map(|a| a.eq_ignore_ascii_case(expected)).unwrap_or(false),
        None => true,
    }
}

async fn attempt(client: &reqwest::Client, job: &Job, progress: &Progress) -> Result<()> {
    if let Some(p) = job.dest.parent() {
        tokio::fs::create_dir_all(p).await?;
    }
    let part = job.dest.with_extension(
        format!("{}.part", job.dest.extension().and_then(|e| e.to_str()).unwrap_or("dl")),
    );
    let existing = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);

    let mut req = client.get(&job.url);
    if existing > 0 {
        req = req.header("Range", format!("bytes={existing}-"));
    }
    let resp = req.send().await?;
    let status = resp.status();
    let (mut file, mut written) = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        (tokio::fs::OpenOptions::new().append(true).open(&part).await?, existing)
    } else if status.is_success() {
        (tokio::fs::File::create(&part).await?, 0) // الخادم تجاهل Range: ابدأ من الصفر
    } else if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(Error::Msg("range not satisfiable, restarting".into()));
    } else {
        return Err(Error::Msg(format!("HTTP {status} for {}", job.url)));
    };
    // ما كان محمّلاً مسبقاً يُحتسب في التقدّم
    progress.done_bytes.fetch_add(written, Ordering::Relaxed);

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        progress.done_bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }
    file.flush().await?;
    drop(file);

    if let Some(expected) = &job.sha1 {
        let actual = sha1_file(&part).await?;
        if !actual.eq_ignore_ascii_case(expected) {
            // الجزء تالف: احذفه حتى لا تعيد المحاولة التالية الاستكمال منه
            let _ = tokio::fs::remove_file(&part).await;
            progress.done_bytes.fetch_sub(written.min(progress.done_bytes.load(Ordering::Relaxed)), Ordering::Relaxed);
            return Err(Error::Sha1 { path: job.dest.display().to_string(), expected: expected.clone(), actual });
        }
    }
    tokio::fs::rename(&part, &job.dest).await?;
    Ok(())
}

/// ينزّل ملفاً واحداً (يتخطاه إن كان سليماً) مع 3 محاولات.
pub async fn fetch(client: &reqwest::Client, job: &Job, progress: &Progress) -> Result<()> {
    if is_valid(job).await {
        if let Some(s) = job.size {
            progress.done_bytes.fetch_add(s, Ordering::Relaxed);
        }
        progress.files_done.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    // ملف موجود لكنه تالف: احذفه قبل التنزيل
    let _ = tokio::fs::remove_file(&job.dest).await;

    let mut last = None;
    for n in 0..3u64 {
        match attempt(client, job, progress).await {
            Ok(()) => {
                progress.files_done.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            Err(e) => {
                last = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(500 * (n + 1))).await;
            }
        }
    }
    Err(last.unwrap())
}

/// ينزّل مجموعة ملفات بتوازي محدود (الافتراضي 16).
pub async fn fetch_all(
    client: &reqwest::Client,
    jobs: Vec<Job>,
    progress: Arc<Progress>,
    concurrency: usize,
) -> Result<()> {
    progress.files_total.fetch_add(jobs.len() as u64, Ordering::Relaxed);
    progress
        .total_bytes
        .fetch_add(jobs.iter().filter_map(|j| j.size).sum(), Ordering::Relaxed);

    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency.max(1)));
    let mut set = tokio::task::JoinSet::new();
    for job in jobs {
        let (c, p, s) = (client.clone(), progress.clone(), sem.clone());
        set.spawn(async move {
            let _permit = s.acquire_owned().await.map_err(|e| Error::Msg(e.to_string()))?;
            fetch(&c, &job, &p).await
        });
    }
    let mut first_err = None;
    while let Some(r) = set.join_next().await {
        match r {
            Ok(Ok(())) => {}
            Ok(Err(e)) => first_err = first_err.or(Some(e)),
            Err(e) => first_err = first_err.or(Some(Error::Msg(e.to_string()))),
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sha1_and_validity() {
        let dir = std::env::temp_dir().join(format!("hrc-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let f = dir.join("a.bin");
        tokio::fs::write(&f, b"abc").await.unwrap();
        // SHA1("abc")
        let good = "a9993e364706816aba3e25717850c26c9cd0d89d";
        assert_eq!(sha1_file(&f).await.unwrap(), good);
        let ok = Job { url: String::new(), dest: f.clone(), sha1: Some(good.into()), size: Some(3) };
        let bad = Job { sha1: Some("00".repeat(20)), ..ok.clone() };
        assert!(is_valid(&ok).await);
        assert!(!is_valid(&bad).await);
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
