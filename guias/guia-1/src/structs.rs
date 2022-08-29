/*#[derive(Debug)]
pub struct Persona {
    nombre: String,
}
impl Persona {
    pub fn new(nombre: String) -> Persona {
        Persona { nombre }
    }
}*/

#[derive(Debug)]
pub enum ErrorDeJuego {
    NoHayMasIntentos,
    ErrorAlLeerElArchivo,
    ImposibleLeerLaLinea,
}
