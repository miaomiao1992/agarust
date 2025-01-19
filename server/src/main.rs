const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_DATABASE_URL: &str = "sqlite:agarust_db.sqlite";

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let file_appender = tracing_appender::rolling::hourly("./", "agarust_server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    let bind_addr = match std::env::var("BIND_ADDR") {
        Ok(bind_addr) => bind_addr,
        Err(_) => DEFAULT_BIND_ADDR.to_string(),
    };
    tracing::info!("bind_addr: {:?}", bind_addr);

    let tcp_listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(tcp_listener) => tcp_listener,
        Err(error) => {
            tracing::error!("TcpListener bind error: {:?}", error);
            return;
        }
    };

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(database_url) => database_url,
        Err(_) => DEFAULT_DATABASE_URL.to_string(),
    };
    tracing::info!("database_url: {:?}", database_url);

    let db_pool = match sqlx::sqlite::SqlitePool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!("SqlitePool connect error: {:?}", error);
            return;
        }
    };

    let hub = agarust_server::hub::Hub::new(db_pool.clone());
    let hub_command_sender = hub.command_sender.clone();
    let hub_task = hub.run();
    tokio::spawn(hub_task);

    while let Ok((tcp_stream, socket_addr)) = tcp_listener.accept().await {
        tracing::info!("tcp_listener accept: {:?}", socket_addr);
        let handle_client_task = agarust_server::handle_tcp_stream(
            tcp_stream,
            socket_addr,
            db_pool.clone(),
            hub_command_sender.clone(),
        );
        tokio::spawn(handle_client_task);
    }
}
