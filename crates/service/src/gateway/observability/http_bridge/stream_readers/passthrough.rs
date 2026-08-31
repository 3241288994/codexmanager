use super::{
    classify_upstream_stream_read_error, inspect_sse_frame_for_protocol, mark_first_response_ms,
    merge_usage, should_emit_keepalive, stream_idle_timed_out, stream_idle_timeout_message,
    stream_reader_disconnected_message, stream_wait_timeout,
    upstream_hint_or_stream_incomplete_message, Arc, Cursor, Mutex, PassthroughSseCollector,
    PassthroughSseProtocol, Read, SseKeepAliveFrame, SseTerminal, UpstreamSseFramePump,
    UpstreamSseFramePumpItem,
};
use crate::gateway::http_bridge::extract_error_hint_from_body;
use std::time::Instant;

pub(crate) struct PassthroughSseUsageReader {
    upstream: UpstreamSseFramePump,
    out_cursor: Cursor<Vec<u8>>,
    usage_collector: Arc<Mutex<PassthroughSseCollector>>,
    keepalive_frame: SseKeepAliveFrame,
    protocol: PassthroughSseProtocol,
    request_started_at: Instant,
    last_upstream_activity: Instant,
    saw_upstream_frame: bool,
    finished: bool,
}

impl PassthroughSseUsageReader {
    /// 函数 `new`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - crate: 参数 crate
    ///
    /// # 返回
    /// 返回函数执行结果
    pub(crate) fn new(
        upstream: reqwest::blocking::Response,
        usage_collector: Arc<Mutex<PassthroughSseCollector>>,
        keepalive_frame: SseKeepAliveFrame,
        protocol: PassthroughSseProtocol,
        request_started_at: Instant,
    ) -> Self {
        Self {
            upstream: UpstreamSseFramePump::new(upstream),
            out_cursor: Cursor::new(Vec::new()),
            usage_collector,
            keepalive_frame,
            protocol,
            request_started_at,
            last_upstream_activity: Instant::now(),
            saw_upstream_frame: false,
            finished: false,
        }
    }

    /// 函数 `update_usage_from_frame`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - lines: 参数 lines
    ///
    /// # 返回
    /// 无
    fn update_usage_from_frame(&self, lines: &[String]) {
        let inspection = inspect_sse_frame_for_protocol(lines, self.protocol);
        if let Ok(mut collector) = self.usage_collector.lock() {
            if let Some(event_type) = inspection.last_event_type {
                collector.last_event_type = Some(event_type);
            }
            // 上游偶尔会用 200 + 正常 data: 帧夹带 "You've hit your usage limit..."
            // 回覆（不走 response.failed）。这类帧里 delta 文本会让 inspection.usage 被
            // 初始化为 Some，直接落到下面的 merge_usage 分支，永远不会触发 terminal。
            // 所以必须在进任何分支前先扫一遍正文；命中 usage-limit 关键字就标 terminal 错误，
            // 让后续 response_finalize 走 failover + cooldown。
            if collector.terminal_error.is_none() {
                if let Some(msg) = extract_usage_limit_from_sse_data(lines) {
                    collector.saw_terminal = true;
                    collector.terminal_error = Some(msg);
                }
            }
            if inspection.usage.is_none() && inspection.terminal.is_none() {
                if collector.upstream_error_hint.is_none() {
                    let raw_frame = lines.concat();
                    let trimmed = raw_frame.trim();
                    let looks_like_sse_frame = lines.iter().any(|line| {
                        let line = line.trim_start();
                        line.starts_with("data:")
                            || line.starts_with("event:")
                            || line.starts_with("id:")
                            || line.starts_with("retry:")
                            || line.starts_with(':')
                    });
                    if !looks_like_sse_frame && !trimmed.is_empty() {
                        collector.upstream_error_hint =
                            extract_error_hint_from_body(400, raw_frame.as_bytes())
                                .or_else(|| Some(trimmed.to_string()));
                    }
                }
                return;
            }
            if let Some(parsed) = inspection.usage {
                merge_usage(&mut collector.usage, parsed);
            }
            if let Some(terminal) = inspection.terminal {
                collector.saw_terminal = true;
                if let SseTerminal::Err(message) = terminal {
                    collector.terminal_error = Some(message);
                }
            }
        }
    }

    /// 函数 `next_chunk`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 返回函数执行结果
    fn next_chunk(&mut self) -> std::io::Result<Vec<u8>> {
        loop {
            match self
                .upstream
                .recv_timeout(stream_wait_timeout(self.last_upstream_activity))
            {
                Ok(UpstreamSseFramePumpItem::Frame(frame)) => {
                    self.last_upstream_activity = Instant::now();
                    self.saw_upstream_frame = true;
                    self.update_usage_from_frame(&frame);
                    mark_first_response_ms(&self.usage_collector, self.request_started_at);
                    return Ok(frame.concat().into_bytes());
                }
                Ok(UpstreamSseFramePumpItem::Eof) => {
                    if let Ok(mut collector) = self.usage_collector.lock() {
                        if !collector.saw_terminal {
                            let hint = collector.upstream_error_hint.clone();
                            collector.terminal_error.get_or_insert_with(|| {
                                upstream_hint_or_stream_incomplete_message(hint.as_deref())
                            });
                        }
                    }
                    self.finished = true;
                    return Ok(Vec::new());
                }
                Ok(UpstreamSseFramePumpItem::Error(err)) => {
                    self.last_upstream_activity = Instant::now();
                    if let Ok(mut collector) = self.usage_collector.lock() {
                        collector
                            .terminal_error
                            .get_or_insert_with(|| classify_upstream_stream_read_error(&err));
                    }
                    self.finished = true;
                    return Ok(Vec::new());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if stream_idle_timed_out(self.last_upstream_activity) {
                        if let Ok(mut collector) = self.usage_collector.lock() {
                            collector
                                .terminal_error
                                .get_or_insert_with(stream_idle_timeout_message);
                        }
                        self.finished = true;
                        return Ok(Vec::new());
                    }
                    if should_emit_keepalive(self.saw_upstream_frame) {
                        return Ok(self.keepalive_frame.bytes().to_vec());
                    }
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    if let Ok(mut collector) = self.usage_collector.lock() {
                        let hint = collector.upstream_error_hint.clone();
                        collector.terminal_error.get_or_insert_with(|| {
                            hint.unwrap_or_else(stream_reader_disconnected_message)
                        });
                    }
                    self.finished = true;
                    return Ok(Vec::new());
                }
            }
        }
    }
}

impl Read for PassthroughSseUsageReader {
    /// 函数 `read`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - buf: 参数 buf
    ///
    /// # 返回
    /// 返回函数执行结果
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let read = self.out_cursor.read(buf)?;
            if read > 0 {
                return Ok(read);
            }
            if self.finished {
                return Ok(0);
            }
            self.out_cursor = Cursor::new(self.next_chunk()?);
        }
    }
}

fn extract_usage_limit_from_sse_data(lines: &[String]) -> Option<String> {
    let mut data_payload = String::new();
    for line in lines {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(rest) = trimmed.strip_prefix("data:") {
            if !data_payload.is_empty() {
                data_payload.push('\n');
            }
            data_payload.push_str(rest.trim_start());
        }
    }
    if data_payload.is_empty() {
        return None;
    }
    crate::account_status::usage_limit_reason_from_message(&data_payload)?;
    Some(data_payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_usage_limit_matches_plain_text_delta() {
        let lines = vec![
            "event: response.output_text.delta\n".to_string(),
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"You've hit your usage limit. To get more access now, send a request to your admin or try again at 7:44 PM.\"}\n".to_string(),
        ];
        let got = extract_usage_limit_from_sse_data(&lines).expect("must match");
        assert!(got.contains("hit your usage limit"));
    }

    #[test]
    fn extract_usage_limit_matches_quota_exceeded_json() {
        let lines = vec![
            "data: {\"error\":{\"code\":\"insufficient_quota\",\"message\":\"quota exceeded\"}}\n"
                .to_string(),
        ];
        assert!(extract_usage_limit_from_sse_data(&lines).is_some());
    }

    #[test]
    fn extract_usage_limit_ignores_unrelated_content() {
        let lines = vec![
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello world\"}\n"
                .to_string(),
        ];
        assert!(extract_usage_limit_from_sse_data(&lines).is_none());
    }

    #[test]
    fn extract_usage_limit_ignores_frames_without_data() {
        let lines = vec!["event: ping\n".to_string(), ": keepalive\n".to_string()];
        assert!(extract_usage_limit_from_sse_data(&lines).is_none());
    }
}
