use std::path::PathBuf;

use anyhow::Result;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{Client, Api, api::entry::Entry};
use log::{error, info};


#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .parse_env(env_logger::Env::default().default_filter_or("info"))
        .init();


    let map_name = std::env::var("MAP_NAME").unwrap_or_default();
    if map_name.is_empty() {
        error!("MAP_NAME is not set");
        std::process::exit(1);
    }

    let mut map_ns = std::env::var("MAP_NS").unwrap_or_default();
    if map_ns.is_empty() {
        let raw_ns = match tokio::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace").await {
            Ok(x) => x.trim().to_string(),
            Err(e) => {
                error!("failed to read k8s namespace from '/var/run/secrets/kubernetes.io/serviceaccount/namespace', maybe set MAP_NS? {e}");
                std::process::exit(1);
            }
        };
        if raw_ns.is_empty() {
            error!("MAP_NS is not set and no K8s NS was autodetected");
            std::process::exit(1);    
        }
        map_ns = raw_ns;
    }
        
    let target_file: PathBuf = std::env::var("TARGET_FILE").unwrap_or_default().into();
    if target_file.as_os_str().is_empty() || target_file.file_name().is_none() {
        error!("TARGET_FILE is invalid");
        std::process::exit(1);
    }
    let mut receiver = really_notify::FileWatcherConfig::new(&target_file, "target").with_parser(|x| String::from_utf8(x)).start();

    let mut map_filename = std::env::var("MAP_FILENAME").unwrap_or_default();
    if map_filename.is_empty() {
        map_filename = target_file.file_name().unwrap().to_string_lossy().into_owned();
    }

    let client = match Client::try_default().await {
        Ok(x) => x,
        Err(e) => {
            error!("failed to init k8s client: {e}");
            std::process::exit(1);
        }
    };
    let maps: Api<ConfigMap> = Api::namespaced(client, &map_ns);

    while let Some(new_content) = receiver.recv().await {
        if let Err(e) = do_map_update(maps.clone(), &map_name, &map_filename, new_content).await {
            error!("failed up update k8s configmap: {e:#}");
        }
    }
}

async fn do_map_update(maps: Api<ConfigMap>, map_name: &str, map_filename: &str, new_content: String) -> Result<()> {
    info!("checking update for configmap {map_name}, file: {map_filename}");
    let mut entry = match maps.entry(&map_name).await? {
        Entry::Occupied(x) => x,
        Entry::Vacant(_) => {
            error!("configmap {map_name} does not exist, skipping update!");
            return Ok(());    
        },
    };

    let current = entry.get_mut();
    if current.data.is_none() {
        current.data = Some(Default::default());
    }

    let data = current.data.as_mut().unwrap();
    if let Some(current_data) = data.get_mut(map_filename) {
        if *current_data != new_content {
            info!("update file in configmap {map_name}: {map_filename}");
            *current_data = new_content;
        } else {
            info!("content not changed, skipping update");
            return Ok(());
        }
    } else {
        info!("inserting new file into configmap {map_name}: {map_filename}");
        data.insert(map_filename.to_string(), new_content);
    }
    entry.commit(&Default::default()).await?;

    Ok(())
}