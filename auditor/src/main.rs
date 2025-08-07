// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use auditor::configuration::{TLSParams, get_configuration};
use auditor::metrics::DatabaseMetricsWatcher;
use auditor::startup::run;
use auditor::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;

use rustls::{RootCertStore, ServerConfig, pki_types::PrivateKeyDer, server::WebPkiClientVerifier};
use rustls_pemfile::{certs, pkcs8_private_keys};

use std::{fs::File, io::BufReader, sync::Arc};

use std::env;

use casbin::{CoreApi, DefaultModel, Enforcer, FileAdapter, MgmtApi, RbacApi};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Read in configuration
    let configuration = get_configuration().expect("Failed to read configuration.");

    // Set up logging
    let subscriber = get_subscriber("AUDITOR".into(), configuration.log_level, std::io::stdout);
    init_subscriber(subscriber);

    // Create a connection pool for the PostgreSQL database
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    // Start background task
    let db_metrics_watcher = DatabaseMetricsWatcher::new(connection_pool.clone(), &configuration)?;
    let db_metrics_watcher_task = db_metrics_watcher.clone();
    // TODO: Don't panic!
    tokio::spawn(async move {
        db_metrics_watcher_task.monitor().await.unwrap();
    });

    if let Some(tls) = configuration.tls_config {
        // tls config if the use_tls option is set to true
        if tls.use_tls {
            let mut cert_store = RootCertStore::empty();

            //rustls::crypto::default_provider().install_default().expect("failed to install default crypto provider");

            // CryptoProvider::install_default();
            rustls::crypto::aws_lc_rs::default_provider()
                .install_default()
                .unwrap();

            if let Err(e) = tls.validate_tls_paths() {
                eprintln!("Configuration error: {e}");
                // Handle the error or return early
            }

            let ca_cert_path = tls.ca_cert_path.as_ref().unwrap();
            let server_key_path = tls.server_key_path.as_ref().unwrap();
            let server_cert_path = tls.server_cert_path.as_ref().unwrap();

            match env::current_dir() {
                Ok(path) => println!("Current directory: {}", path.display()),
                Err(e) => eprintln!("Error getting current directory: {e}"),
            }

            // import CA cert
            let ca_cert = &mut BufReader::new(File::open(ca_cert_path)?);
            let ca_cert = certs(ca_cert).collect::<Result<Vec<_>, _>>().unwrap();

            for cert in ca_cert {
                cert_store.add(cert).expect("root CA not added to store");
            }

            // set up client authentication requirements
            let client_auth = WebPkiClientVerifier::builder(Arc::new(cert_store))
                .build()
                .unwrap();
            let config = ServerConfig::builder().with_client_cert_verifier(client_auth);

            // import server cert and key
            let cert_file = &mut BufReader::new(File::open(server_cert_path)?);
            let key_file = &mut BufReader::new(File::open(server_key_path)?);

            let cert_chain = certs(cert_file).collect::<Result<Vec<_>, _>>().unwrap();
            let mut keys = pkcs8_private_keys(key_file)
                .map(|key| key.map(PrivateKeyDer::Pkcs8))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let config = config.with_single_cert(cert_chain, keys.remove(0)).unwrap();

            let tls_params = TLSParams {
                config,
                https_addr: tls.https_addr,
                https_port: tls.https_port,
                use_tls: tls.use_tls,
            };

            let enforce_rbac: bool = configuration
                .rbac_config
                .as_ref()
                .map(|r| r.enforce_rbac)
                .unwrap_or(false);

            if let Some(rbac_config) = &configuration.rbac_config
                && rbac_config.enforce_rbac
            {
                let m = DefaultModel::from_file("model.conf").await.unwrap();

                let _writer_file = File::create("policy.csv")?;

                let adapter = FileAdapter::new("policy.csv");

                let mut enforcer = Enforcer::new(m, adapter).await.unwrap();

                enforcer
                    .add_policies(rbac_config.base_policies.clone())
                    .await
                    .unwrap();

                enforcer.save_policy().await.unwrap();

                if let Some(monitoring_role_cn) = &rbac_config.monitoring_role_cn {
                    for item in monitoring_role_cn.iter() {
                        enforcer
                            .add_role_for_user(item, "monitoring_role", None)
                            .await
                            .unwrap();

                        enforcer.save_policy().await.unwrap();
                    }
                }

                if let Some(write_access_cn) = &rbac_config.write_access_cn {
                    for item in write_access_cn.iter() {
                        enforcer
                            .add_role_for_user(item, "write_access_base", None)
                            .await
                            .unwrap();

                        enforcer.save_policy().await.unwrap();
                    }
                }

                if let Some(read_access_cn) = &rbac_config.read_access_cn {
                    for item in read_access_cn.iter() {
                        enforcer
                            .add_role_for_user(item, "read_access_base", None)
                            .await
                            .unwrap();

                        enforcer.save_policy().await.unwrap();
                    }
                }

                if let Some(data_access_rules) = &rbac_config.data_access_rules {
                    for item in data_access_rules {
                        for (meta_id, meta_values) in item.meta_info.iter() {
                            for meta_value in meta_values {
                                enforcer
                                    .add_policy(vec![
                                        "meta".to_string(),
                                        meta_id.to_string(),
                                        meta_value.to_string(),
                                    ])
                                    .await
                                    .unwrap();

                                enforcer.save_policy().await.unwrap();

                                enforcer
                                    .add_role_for_user(
                                        &item.reader_cn.clone(),
                                        "read_access_base",
                                        None,
                                    )
                                    .await
                                    .unwrap();

                                enforcer.save_policy().await.unwrap();
                            }
                        }
                    }
                }
            }

            run(
                configuration.application.addr,
                configuration.application.port,
                configuration.application.web_workers,
                connection_pool,
                db_metrics_watcher,
                Some(tls_params),
                enforce_rbac,
                configuration.ignore_record_exists_error,
                configuration.archival_config,
            )
            .await?
            .await?;
        } else {
            // Start server
            run(
                configuration.application.addr,
                configuration.application.port,
                configuration.application.web_workers,
                connection_pool,
                db_metrics_watcher,
                None,
                false,
                configuration.ignore_record_exists_error,
                configuration.archival_config,
            )
            .await?
            .await?;
        }
    } else {
        // Start server
        run(
            configuration.application.addr,
            configuration.application.port,
            configuration.application.web_workers,
            connection_pool,
            db_metrics_watcher,
            None,
            false,
            configuration.ignore_record_exists_error,
            configuration.archival_config,
        )
        .await?
        .await?;
    }

    Ok(())
}
