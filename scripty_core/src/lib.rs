pub fn start() {
    load_config();

    scripty_audio::init_stt();

    let rt = get_tokio_rt();

    rt.block_on(async_init());
    rt.spawn(scripty_webserver::entrypoint());
    rt.block_on(scripty_commands::entrypoint());
}

async fn async_init() {
    scripty_db::init_db().await;
}

fn get_tokio_rt() -> tokio::runtime::Runtime {
    let cfg = scripty_config::get_config();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(
            (num_cpus::get() as f32 * (1.0 - cfg.pct_stt_threads))
                .floor()
                .min(1.0) as usize,
        )
        .enable_all()
        .build()
        .expect("failed to build new tokio rt")
}

fn load_config() {
    let cfg_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./config.toml".to_string());
    println!("reading cfg at {}", cfg_path);

    scripty_config::load_config(&cfg_path);
}
