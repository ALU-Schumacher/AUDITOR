use actix_web::HttpMessage;

use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    middleware::Next,
};

use casbin::{CoreApi, Enforcer, MgmtApi};

use rustls::pki_types::CertificateDer;
use std::collections::HashMap;
use x509_parser::prelude::*;

use serde::Deserialize;
use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::web;

pub struct CommonNameFromClientCert {
    pub common_name: String,
}

#[derive(Deserialize, Debug)]
pub struct RbacAccess {
    pub meta: Option<HashMap<String, MetaOuter>>,
}

#[derive(Deserialize, Debug)]
pub struct MetaOuter {
    pub c: Option<Vec<String>>,
    pub dnc: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct MetaAccess {
    pub meta: HashMap<String, Vec<String>>,
}

pub async fn rbac(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // pre-processing

    let is_https = req.connection_info().scheme().eq_ignore_ascii_case("https");

    let enforce_rbac = req
        .app_data::<web::Data<bool>>()
        .ok_or_else(|| ErrorInternalServerError("enforce_rbac bool value is missing"))?
        .clone();

    if is_https && **enforce_rbac {
        let enforcer_data = req
            .app_data::<web::Data<Option<Arc<Enforcer>>>>()
            .ok_or_else(|| ErrorInternalServerError("Enforcer option missing"))?;

        // Unwrap the Option to get the Arc<Enforcer>
        let enforcer = enforcer_data
            .as_ref()
            .clone()
            .ok_or_else(|| ErrorInternalServerError("Enforcer not initialized"))?;

        let client_cert = req.conn_data::<CertificateDer<'static>>();

        let common_name = if let Some(cert) = client_cert {
            match parse_x509_certificate(cert.as_ref()) {
                Ok((_, x509)) => {
                    let subject = x509.subject();
                    subject
                        .iter_common_name()
                        .next()
                        .map(|cn| cn.as_str().unwrap_or("unknown"))
                        .unwrap_or("missing")
                        .to_string()
                }
                Err(_) => "invalid_cert".to_string(),
            }
        } else {
            "no_cert".to_string()
        };

        req.extensions_mut().insert(CommonNameFromClientCert {
            common_name: common_name.clone(),
        });

        let path = req.path().to_string();
        let method = req.method().as_str().to_string();

        let query_string = req.query_string();

        let meta_struct: RbacAccess = serde_qs::from_str(query_string).unwrap();

        let meta_policies = enforcer.get_filtered_policy(0, vec!["meta".to_string()]);

        if !meta_policies.is_empty() {
            if let Some(meta_info) = meta_struct.meta {
                for (key, outer) in meta_info.iter() {
                    if let Some(values) = &outer.c {
                        for value in values {
                            let permitted = enforcer
                                .enforce(("meta".to_string(), key.clone(), value.clone()))
                                .map_err(actix_web::error::ErrorInternalServerError)?;

                            if !permitted {
                                return Err(actix_web::error::ErrorForbidden(
                                    "Access denied by RBAC policy",
                                ));
                            }
                        }
                    }

                    if let Some(values) = &outer.dnc {
                        for value in values {
                            let permitted = enforcer
                                .enforce(("meta".to_string(), key.clone(), value.clone()))
                                .map_err(actix_web::error::ErrorInternalServerError)?;

                            if !permitted {
                                return Err(actix_web::error::ErrorForbidden(
                                    "Access denied by RBAC policy",
                                ));
                            }
                        }
                    }
                }
            }

            let mut meta: HashMap<String, Vec<String>> = HashMap::new();
            for policy in &meta_policies {
                if policy.len() >= 3 && policy[0] == "meta" {
                    let key = &policy[1];
                    let value = &policy[2];

                    meta.entry(key.clone()).or_default().push(value.clone());
                }
            }

            req.extensions_mut().insert(meta);
        }

        // Check if the user has permission
        let permitted = enforcer.enforce((common_name.as_str(), path.as_str(), method.as_str()));

        match permitted {
            Ok(true) => {
                // User has permission, proceed with the request
                let res = next.call(req).await?;
                Ok(res)
            }
            _ => {
                // User doesn't have permission
                Err(ErrorForbidden("Insufficient permissions"))
            }
        }
    } else {
        let res = next.call(req).await?;
        Ok(res)
    }
}
