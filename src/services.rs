use crate::i18n::{t, Lang};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Checking,
    Online,
    Offline,
    Error,
}

impl ServiceStatus {
    pub fn label_for(&self, lang: Lang) -> &'static str {
        let key = match self {
            ServiceStatus::Checking => "service_checking",
            ServiceStatus::Online => "service_online",
            ServiceStatus::Offline => "service_offline",
            ServiceStatus::Error => "service_error",
        };
        t(lang, key)
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            ServiceStatus::Checking => "text-yellow-400",
            ServiceStatus::Online => "text-emerald-400",
            ServiceStatus::Offline => "text-rose-400",
            ServiceStatus::Error => "text-orange-400",
        }
    }

}
