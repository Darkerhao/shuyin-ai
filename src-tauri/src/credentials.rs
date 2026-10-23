/// No command exposes secret reads to the WebView. The service is the app identifier,
/// so independently installed verification products have separate credential entries.
pub trait Credentials {
    fn get(&self, reference: &str) -> Result<Option<String>, String>;
    fn set(&self, reference: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, reference: &str) -> Result<(), String>;
}

pub struct SystemCredentials(pub String);
impl SystemCredentials {
    fn entry(&self, reference: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.0, reference)
            .map_err(|_| "系统凭据库不可用，请解锁系统钥匙串或凭据管理器。".into())
    }
}
impl Credentials for SystemCredentials {
    fn get(&self, reference: &str) -> Result<Option<String>, String> {
        match self.entry(reference)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("无法读取系统凭据，请检查系统凭据库是否已解锁。".into()),
        }
    }
    fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
        self.entry(reference)?
            .set_password(secret)
            .map_err(|_| "密钥未能保存到系统凭据库，模型配置未保存。".into())
    }
    fn delete(&self, reference: &str) -> Result<(), String> {
        match self.entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("无法删除系统凭据，请解锁凭据库后重试。".into()),
        }
    }
}
