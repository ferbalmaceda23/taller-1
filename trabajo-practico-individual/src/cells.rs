use std::fmt::Display;

/// Enum que implementa la celda del buscaminas.
#[derive(PartialEq, Eq, Debug)]
pub enum Cell {
    /// Representa una mina en la celda.
    Mine,
    /// Representa una celda vacía donde luego se almacenará el número de minas adyacentes.
    Empty(u8),
    /// Representa un caracter inválido cargado en el archivo.
    Invalid,
}
/// Implementación del trait Display para la celda.
impl Display for Cell {
    /// Función que devuelve el caracter que representa la celda
    /// para su impresión en consola.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Cell::Mine => write!(f, "*"),
            Cell::Empty(i) => {
                if *i == 0 {
                    write!(f, ".")
                } else {
                    write!(f, "{}", i)
                }
            }
            Cell::Invalid => Err(std::fmt::Error),
        }
    }
}
