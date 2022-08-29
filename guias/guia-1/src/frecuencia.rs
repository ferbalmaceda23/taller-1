use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::collections::HashMap;
use super::structs::ErrorDeJuego;

pub fn frecuencia() {
    let path: String = "src/files/texto.txt".to_string();
    let mut hash: HashMap<String, u8> = HashMap::new();

    match obtener_lineas(&path) {
        Ok(lineas) => {
            procesar_lineas(&lineas, &mut hash);
            mostrar_palabras(&hash);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

fn mostrar_palabras(hash: &HashMap<String, u8>) {
    let mut vec_elementos: Vec<_> = hash.iter().collect();
    vec_elementos.sort_by(|e1, e2| e1.1.cmp(e2.1).reverse());

    for (palabra, cantidad) in vec_elementos.iter() {
        println!("{} -> {}", palabra, cantidad);
    }
}

fn procesar_lineas(lineas: &[String], hash: &mut HashMap<String, u8>) {
    for linea in lineas.iter() {
        let palabras: Vec<String> = linea
            .split(' ')
            .map(|s| {
                s.to_string().to_lowercase().replace(
                    &['(', ')', ',', '\"', '.', ';', ':', '\'', '!', '?', '¡', '¿'],
                    "",
                )
            })
            .collect();
        for palabra in palabras.iter() {
            if hash.contains_key(palabra) {
                match hash.get(palabra) {
                    Some(cantidad) => {
                        hash.insert(palabra.to_string(), *cantidad + 1);
                    }
                    None => {}
                }
            } else {
                hash.insert(palabra.to_string(), 1);
            }
        }
    }
}

fn obtener_lineas(filepath: &str) -> Result<Vec<String>, ErrorDeJuego> {
    match File::open(filepath) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut lineas: Vec<String> = Vec::new();

            for line in reader.lines() {
                match line {
                    Ok(line) => lineas.push(line),
                    Err(_e) => return Err(ErrorDeJuego::ImposibleLeerLaLinea),
                }
            }

            Ok(lineas)
        }
        Err(_e) => Err(ErrorDeJuego::ErrorAlLeerElArchivo),
    }
}
