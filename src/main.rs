// Prevents additional console window on Windows in release, DO NOT REMOVE!!

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use tauri::State;
use tauri::{AppHandle, Emitter};
use std::io::Read;
use std::{net::TcpListener, net::TcpStream, io::prelude::*, thread};
use std::sync::Mutex;

fn main() {

    tauri::Builder::default()
        .manage(Mutex::new(AppState { stream : None, username : String::new() }))
        .invoke_handler(tauri::generate_handler![server_listen, client_connect, send ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn server_listen(app : AppHandle, state : State<Mutex<AppState>>, username : String){

    let socket: TcpListener = TcpListener::bind("127.0.0.1:0").unwrap();

    println!("{}", socket.local_addr().unwrap().to_string());

    let result: (TcpStream, std::net::SocketAddr) = socket.accept().unwrap();
    let mut stream : TcpStream = result.0;

    get_data(app, stream.try_clone().unwrap());

    let message: Message = Message {
        first_connect : true,
        username : username.clone(),
        message : format!("{} connected", username.clone()),
        is_emoji : false
    };

    let payload: String = serde_json::to_string(&message).unwrap();

    stream.write(format!("{:010}{}", payload.len(), payload).as_bytes()).unwrap();

    *state.lock().unwrap() = AppState { stream : Some(stream), username : username };
}

#[tauri::command]
fn client_connect(app : AppHandle, state : State<Mutex<AppState>>, host : String, username : String){

    let mut stream: TcpStream = TcpStream::connect(format!("{}", host)).unwrap();

    let message: Message = Message {
        first_connect : true,
        username : username.clone(),
        message : format!("{} connected", username.clone()),
        is_emoji : false
    };

    let payload: String = serde_json::to_string(&message).unwrap();

    stream.write(format!("{:010}{}", payload.len(), payload).as_bytes()).unwrap();

    get_data(app, stream.try_clone().unwrap());

    *state.lock().unwrap() = AppState { stream : Some(stream), username : username };
}

#[tauri::command(rename_all = "snake_case")]
fn send(state : State<'_, Mutex<AppState>>, message : String, is_emoji : bool){ 

    let state: std::sync::MutexGuard<'_, AppState> = state.lock().unwrap();

    let option: &Option<TcpStream> = &state.stream;
    let username: &String = &state.username;

    match option.as_ref(){
        Some(mut stream) => {

            let message: Message = Message {
                first_connect : false,
                username : username.into(),
                message,
                is_emoji 
            };

            let payload: String = serde_json::to_string(&message).unwrap();

            stream.write(format!("{:010}{}",  payload.len(), payload).as_bytes()).unwrap();
        },
        None => {}
    }
}

fn get_data(app : AppHandle, mut stream : TcpStream){
    thread::spawn(move || {
        loop {
            let mut buffer: [u8; 10] = [0; 10];
            stream.read(&mut buffer).unwrap();
            
            match std::str::from_utf8(&buffer).unwrap().to_string().parse::<usize>(){
                Ok(size) => {

                    let mut total_read: usize = 0;
                    let mut string_builder: String = String::new();

                    loop {
                        let mut buffer: [u8; 10] = [0; 10];
                        let read: usize = stream.read(&mut buffer).unwrap();

                        string_builder.push_str(std::str::from_utf8(&buffer).unwrap().trim_end_matches('\0'));
                        total_read += read;

                        if total_read == size{
                            break;
                        }
                    }

                    app.emit("message", &string_builder).unwrap();
                },
                _ => {}
            }
        }
    });
}

#[derive(Debug)]
struct AppState {
    stream : Option<TcpStream>,
    username : String
}

#[derive(Serialize)]
struct Message {
    first_connect : bool,
    username : String,
    message : String,
    is_emoji : bool
}