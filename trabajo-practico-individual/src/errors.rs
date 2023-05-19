use std::io::ErrorKind;
use std::{convert::From, io::Error};

/// Enum que representa los errores que pueden
/// ocurrir en el programa.
#[derive(PartialEq, Eq, Debug)]
pub enum CustomError {
    /// Error que se produce cuando no existe la ruta del archivo.
    FileNotFound,
    /// Error que se produce cuando se envia un archivo vacio.
    EmptyBoard,
    /// Error que se produce cuando el largo de filas no es igual en el archivo.
    InvalidBoard,
    /// Error que se produce cuando el archivo contiene un caracter distinto.
    /// de '*' o '.'
    InvalidChar,
    /// Error que se produce cuando no se ingresan argumentos en la línea de comandos.
    NotEnoughArgs,
    /// Error que contempla otros tipos de errores del programa ajenos al buscaminas.
    Other,
}

/// Implementación del trait From para cuando se levante un
/// error distinto del tipo CustomError y poder interpretarlo
/// como un tipo de CustomError.
impl From<Error> for CustomError {
    /// Funcion que matchea un error distinto de CustomError
    /// con uno de los tipos de CustomError.
    fn from(e: Error) -> Self {
        match e.kind() {
            ErrorKind::NotFound => CustomError::FileNotFound,
            _ => CustomError::Other,
        }
    }
}
