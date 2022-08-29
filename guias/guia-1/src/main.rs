mod structs;
mod ahorcado;
mod frecuencia;

use std::io::stdin;
use ahorcado::ahorcado;
use frecuencia::frecuencia;

fn main() {
    buscador_full_text();
    consultar();
}

fn buscador_full_text() {
    //aca iria un buscador full text, SI TUVIERA UNO!!!11!!
}

//-----------------------------------------------
fn consultar() {
    por_ahorcado();
    por_frecuencia();
}

fn por_ahorcado() {
    let respuesta = preguntar("¿Desea jugar al ahorcado? (s/n)");
    if respuesta == "s" || respuesta == "S" {
        ahorcado();
    }
}

fn por_frecuencia() {
    let respuesta = preguntar("¿Desea contar frecuencia de palabras? (s/n)");
    if respuesta == "s" || respuesta == "S" {
        frecuencia();
    }
}

fn preguntar(pregunta: &str) -> String {
    println!("{}", pregunta);
    let mut respuesta = String::new();
    stdin().read_line(&mut respuesta).unwrap();
    respuesta.truncate(respuesta.len() - 1);
    respuesta
}