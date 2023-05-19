# Proyecto IRC - Grupo Rust Beef
* Fernando Balmaceda
* Nicolás Pinto
* Mateo Craviotto
* Agustina Bocaccio

## How to run the application
#### Run main server 
```
 cargo run -p server -- <port> main_server 
 ```
For example: 
``` 
cargo run -p server -- 8080 main_server
 ```

#### Run server child
``` 
cargo run -p server -- <child_port> <child_name> <parent_name> <parent_ip>  <parent_port>
 ```
For example:
```
 cargo run -p server -- 8081 child_server main_server 0.0.0.0 8080
  ```
  Then you need to enter SERVER <child_name> <hopcount>


#### Run client in terminal
``` 
cargo run -p client -- <server_ip> <server_port> 
```
For example:
``` 
cargo run -p client -- 127.0.0.1 8080 
 ```


#### Run client with GUI
```
 cargo run -p client
  ```
