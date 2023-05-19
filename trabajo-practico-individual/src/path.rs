use super::errors::CustomError;
use std::env;
use std::path::Path;

/// Función que devuelve el path del archivo de entrada
/// recibido en la línea de comandos. En caso de no recibir
/// ningún argumento, devuelve un error del tipo
/// CustomError::NotEnoughArgs.
pub fn get_path() -> Result<String, CustomError> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Error, no se ingreso la ruta del archivo. Ver README.md");
        return Err(CustomError::NotEnoughArgs);
    }
    Ok(args[1].to_string())
}

/// Función que chequea la existencia de la ruta del archivo
/// de obtenida. En caso de no existir, devuelve un error del
/// tipo CustomError::FileNotFound.
pub fn check_path(path: &String) -> Result<(), CustomError> {
    if !Path::new(path).exists() {
        println!("Error, el archivo no existe");
        return Err(CustomError::FileNotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_additional_args_raise_error() {
        let result = get_path();
        assert_eq!(result, Err(CustomError::NotEnoughArgs));
    }

    #[test]
    fn correct_path_return_ok() {
        let result = check_path(&"src/files/buscaminas_1.txt".to_string());
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn wrong_path_raise_error() {
        let result = check_path(&"".to_string());
        assert_eq!(result, Err(CustomError::FileNotFound));
    }
}
