/// Fuente de tiempo abstracta (segundos Unix). Permite pruebas deterministas.
pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}
