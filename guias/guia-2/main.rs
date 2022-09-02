fn main() {
    primer_codigo();

    let a: String = drip_drop(); // segundo codigo
    println!("{}", a);

    tercer_codigo();
}

//--------------------------------------------------------------------------

/* codigo que esta mal
fn main() {
    let mut s = String::from("hola");
    let ref1 = &s;
    let ref2 = &ref1;
    let ref3 = &ref2;
    s = String::from("chau");
    println!("{}", ref3.to_uppercase());
}
*/

//codigo que esta bien
fn primer_codigo() {
    let mut s = String::from("hola");
    let mut ref1 = &mut s;
    let mut ref2 = &mut ref1;
    let ref3 = &mut ref2;
    ***ref3 = String::from("chau");
    println!("{}", ref3.to_uppercase());
}

//--------------------------------------------------------------------------

/* codigo que esta mal
fn drip_drop() -> &String {
    let s = String::from("hello world!");
    return &s;
}
*/

// codio que esta bien
fn drip_drop() -> String {
    String::from("hello world!")
}

//--------------------------------------------------------------------------

/* codigo que esta mal
fn main() {
    let s1 = String::from("hola");
    let mut v = Vec::new();
    v.push(s1);
    let s2: String = v[0];
    println!("{}", s2);
}
*/

// codigo que esta bien
fn tercer_codigo() {
    let s1 = String::from("hola");
    let v = vec![s1];
    let s2: &String = &v[0];
    println!("{}", s2);
}