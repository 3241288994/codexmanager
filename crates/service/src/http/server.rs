pub fn start_http(addr: &str) -> std::io::Result<()> {
    let server = tiny_http::Server::http(addr)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    for request in server.incoming_requests() {
        crate::http::backend_router::handle_backend_request(request);
        if crate::shutdown_requested() {
            break;
        }
    }
    Ok(())
}
