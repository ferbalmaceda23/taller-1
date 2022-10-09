/*
Un threadpool mantiene varios hilos de ejecución (threads) en espera
de que el programa supervisor asigne tareas para su ejecución simultánea.
Al mantener un grupo de threads, el modelo aumenta el rendimiento y
evita la latencia en la ejecución debido a la frecuente creación y
destrucción de threads para tareas de corta duración.

En este ejercicio se debe armar un threadpool sencillo haciendo uso de
las herramientas para computación concurrente que nos provee la biblioteca
estándar de Rust.

Para distribuir las tareas a realizar entre los threads del pool se puede
utilizar una cola concurrente.

Consideraciones a tener en cuenta:

La estructura de datos utilizada para distribuir el trabajo. ¿Que se hace
cuando una tarea enviada al threadpool provoca que un thread muera? Esta
situación no debería afectar a otros threads. Ademas tras la muerte de un
thread, se debe crear otro de forma de que la cantidad total de threads
en el pool no cambie. Cuando la threadpool es terminada o sale de scope
todos los threads deberian finalizar. El fragmento de código 2 muestra un
ejemplo de uso:

fn main() {
    let pool = ThreadPool::new(4);
    for i in 0..4 {
        pool.spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(250 * i));
            println!("This is Task {}", i);
        });
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
}
*/



fn main(){

}
