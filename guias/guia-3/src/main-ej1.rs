/*
El fragmento de código 1 hace uso de unsafe para
poder mutar una variable global. Esto introduce
condiciones de carrera sobre los datos (data races)
que provocan que el programa falle de manera imprevista
al correrlo repetidas veces. Es decir, el problema se
presenta en alguno de los posibles escenarios de ejecución.

Corregir el programa haciendo uso de las abstracciones
que provee Rust para el manejo de la concurrencia de
manera que no se produzcan errores.
*/

use std::thread;
use std::sync::RwLock;

struct Account(i32);

impl Account {
    fn deposit(&mut self, amount: i32) {
        println!("op: deposit {}, available funds: {:?}", amount, self.0);
        self.0 += amount;
    }

    fn withdraw(&mut self, amount: i32) {
        println!("op: withdraw {}, available funds: {}", amount, self.0);
        if self.0 >= amount {
            self.0 -= amount;
        } else {
            panic!("Error: Insufficient funds.")
        }
    }

    fn balance(&self) -> i32 {
        self.0
    }
}

static ACCOUNT: RwLock<Account> = RwLock::new(Account(0));

fn main() {
    let customer1_handle = thread::spawn(move || {
        match ACCOUNT.write() {
            Ok(mut account) => {
                account.deposit(100);
            },
            Err(e) => println!("Error: {:?}", e),
        }
    });

    let customer2_handle = thread::spawn(move || {
        match ACCOUNT.write() {
            Ok(mut account) => {
                account.withdraw(30);
            },
            Err(e) => println!("Error: {:?}", e),
        }
    });

    let customer3_handle = thread::spawn(move || {
        match ACCOUNT.write() {
            Ok(mut account) => {
                account.deposit(60);
            },
            Err(e) => println!("Error: {:?}", e),
        }
    });

    let customer4_handle = thread::spawn(move || {
        match ACCOUNT.write() {
            Ok(mut account) => {
                account.withdraw(70);
            },
            Err(e) => println!("Error: {:?}", e),
        }
    });

    let handles = vec![
        customer1_handle,
        customer2_handle,
        customer3_handle,
        customer4_handle,
    ];

    for handle in handles {
        match handle.join() {
            Ok(_) => (),
            Err(e) => println!("Error: {:?}", e),
        }
    }

    let savings = match ACCOUNT.read() {
        Ok(account) => account.balance(),
        Err(e) => {
            println!("Error: {:?}", e);
            0
        },
    };

    println!("Balance: {:?}", savings);
}