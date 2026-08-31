/// 函数 `split_http_response`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - buf: 参数 buf
///
/// # 返回
/// 返回函数执行结果
fn split_http_response(buf: &str) -> Option<(&str, &str)> {
    if let Some((headers, body)) = buf.split_once("\r\n\r\n") {
        return Some((headers, body));
    }
    if let Some((headers, body)) = buf.split_once("\n\n") {
        return Some((headers, body));
    }
    None
}

/// 函数 `response_uses_chunked`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - headers: 参数 headers
///
/// # 返回
/// 返回函数执行结果
fn response_uses_chunked(headers: &str) -> bool {
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("transfer-encoding")
            && value.to_ascii_lowercase().contains("chunked")
    })
}

/// 函数 `decode_chunked_body`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
fn decode_chunked_body(raw: &str) -> Result<String, String> {
    let bytes = raw.as_bytes();
    let mut cursor = 0usize;
    let mut out = Vec::<u8>::new();

    loop {
        let Some(line_end_rel) = bytes[cursor..].windows(2).position(|w| w == b"\r\n") else {
            return Err("Invalid chunked body: missing chunk size line".to_string());
        };
        let line_end = cursor + line_end_rel;
        let line = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|err| format!("Invalid chunked body: chunk size is not utf8 ({err})"))?;
        let size_hex = line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| format!("Invalid chunked body: bad chunk size '{size_hex}'"))?;
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let end = cursor.saturating_add(size);
        if end + 2 > bytes.len() {
            return Err("Invalid chunked body: truncated chunk payload".to_string());
        }
        out.extend_from_slice(&bytes[cursor..end]);
        if &bytes[end..end + 2] != b"\r\n" {
            return Err("Invalid chunked body: missing chunk terminator".to_string());
        }
        cursor = end + 2;
    }

    String::from_utf8(out).map_err(|err| format!("Invalid chunked body utf8 payload: {err}"))
}

/// 函数 `parse_http_body`
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
pub(crate) fn parse_http_body(buf: &str) -> Result<String, String> {
    let Some((headers, body_raw)) = split_http_response(buf) else {
        return Ok(buf.to_string());
    };
    if response_uses_chunked(headers) {
        decode_chunked_body(body_raw)
    } else {
        Ok(body_raw.to_string())
    }
}
