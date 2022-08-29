use std::fs::File;
use std::io::stdin;
use std::io::BufRead;
use std::io::BufReader;
use super::structs::ErrorDeJuego;

pub fn ahorcado() {
    println!("Bienvenido al ahorcado de FIUBA!\n");

    let path: String = "src/files/palabras.txt".to_string();

    match obtener_palabras(&path) {
        Ok(palabras) => {
            for palabra in palabras.iter() {
                let vec_palabra: Vec<String> = palabra.chars().map(|c| c.to_string()).collect();
                let mut palabra_vacia: Vec<String> = vec!['_'.to_string(); palabra.len()];
                let mut letras_adivinadas: Vec<String> = Vec::new();
                let mut letras_falladas: Vec<String> = Vec::new();

                match jugar(
                    &mut palabra_vacia,
                    &mut letras_adivinadas,
                    &mut letras_falladas,
                    &vec_palabra,
                ) {
                    Ok(()) => println!("Ganaste! :D\n"),
                    Err(e) => {
                        println!("Perdiste! :(");
                        println!("{:?}\n", e);
                        println!("La palabra era: {}", palabra);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

fn jugar(
    palabra_vacia: &mut [String],
    letras_adivinadas: &mut Vec<String>,
    letras_falladas: &mut Vec<String>,
    vec_palabra: &Vec<String>,
) -> Result<(), ErrorDeJuego> {
    let stdin = stdin();
    let mut intentos: u8 = 8;

    while intentos > 0 && palabra_vacia.contains(&'_'.to_string()) {
        let mut letra = String::new();

        mostrar_info(palabra_vacia, letras_adivinadas, letras_falladas, intentos);

        match stdin.read_line(&mut letra) {
            Ok(_) => {
                letra.truncate(letra.len() - 1);
                chequear_letra(
                    palabra_vacia,
                    vec_palabra,
                    letras_adivinadas,
                    letras_falladas,
                    &letra,
                    &mut intentos,
                );
            }
            Err(e) => {
                println!("No se pudo leer la letra.");
                println!("Error: {:?}", e);
            }
        }

        println!();
    }

    if intentos == 0 {
        Err(ErrorDeJuego::NoHayMasIntentos)
    } else {
        Ok(())
    }
}

fn chequear_letra(
    palabra_vacia: &mut [String],
    vec_palabra: &Vec<String>,
    letras_adivinadas: &mut Vec<String>,
    letras_falladas: &mut Vec<String>,
    letra: &String,
    intentos: &mut u8,
) {
    if vec_palabra.contains(letra) {
        if !letras_adivinadas.contains(letra) {
            letras_adivinadas.push(String::from(letra));
            actualizar_palabra_vacia(vec_palabra, palabra_vacia, letra);
        }
    } else if !letras_falladas.contains(letra) && !letra.is_empty() {
        letras_falladas.push(String::from(letra));
        *intentos -= 1
    }
}

fn actualizar_palabra_vacia(palabra: &Vec<String>, palabra_vacia: &mut [String], letra: &String) {
    for i in 0..palabra.len() {
        if &palabra[i] == letra {
            palabra_vacia[i] = String::from(letra);
        }
    }
}

fn mostrar_info(
    palabra_vacia: &[String],
    letras_adivinadas: &[String],
    letras_falladas: &[String],
    intentos: u8,
) {
    let palabra_vacia_string = &palabra_vacia.join(" ");
    let letras_adivinadas_string = &letras_adivinadas.join(" ");
    let letras_falladas_string = &letras_falladas.join(" ");

    println!("La palabra hasta el momento es: {}", palabra_vacia_string);
    println!(
        "Adivinaste las siguientes letras: {}",
        letras_adivinadas_string
    );
    println!("Fallaste las siguientes letras: {}", letras_falladas_string);
    println!("Te quedan {} intentos.", intentos);
    println!("Ingresa una letra:");
}

fn obtener_palabras(filepath: &str) -> Result<Vec<String>, ErrorDeJuego> {
    match File::open(filepath) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut palabras: Vec<String> = Vec::new();

            for line in reader.lines() {
                match line {
                    Ok(line) => palabras.push(line),
                    Err(_e) => return Err(ErrorDeJuego::ImposibleLeerLaLinea),
                }
            }

            Ok(palabras)
        }
        Err(_e) => Err(ErrorDeJuego::ErrorAlLeerElArchivo),
    }
}
