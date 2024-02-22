use async_std::io::BufReader;
use async_std::prelude::*;
use async_std::{task,io,net};
use std::sync::Arc;
use futures_lite::future::FutureExt;
use std;

use rustChat::utils::{self,ChatResult};
use rustChat::{Client,Server};

fn get_value(mut input: &str) -> Option<(&str,&str)> {
    input= input.trim_start();

    if input.is_empty(){
        return None
    }
    match input.find(char::is_whitespace) {
        Some(whitespace) => Some((&input[0..whitespace],&input[whitespace..])),
        None => Some((input,""))
    }
}
fn parse_input(line: &str) -> Option<Client> {
    let (input,remainder) = get_value(line)?;
    let pid = std::process::id();
    return match input {
        "join" => {
            let (chat, remainder) = get_value(remainder)?;
            if !remainder.trim_start().is_empty() {
                return None
            }
            Some(Client::Join { chat_name: Arc::new(chat.to_string()) })
        }
        "post" => {
            let (chat, remainder) = get_value(remainder)?;
            let message = format!("the client {} says: {}\n",
                                  pid,
                                  remainder.trim_start().to_string());
            Some(Client::Post { chat_name: Arc::new(chat.to_string()), message: Arc::new(message) })
        }
        "leave" => {
            std::process::exit(0)
        }
        _ => {
            println!("Unrecognized Input: {:?}", line);
            None
        }
    };
}


async fn send(mut send: net::TcpStream) -> ChatResult<()>{
    println!("Options: \
    \njoin CHAT (creates one if chat does not exists)\
    \npost CHAT MESSAGE (post a message in the chat) \
    \nleave exists the program");
    

    let mut option = BufReader::new(io::stdin()).lines();

    while let Some(option_result) = option.next().await{
        let opt = option_result?;
        let req = match parse_input(&opt) {
            Some(req) => req,
            None => continue,
        };
        utils::send_json(&mut send,&req).await?;
        send.flush().await?;
    }


    Ok(())
}



async fn messages(server: net::TcpStream) -> ChatResult<()>{
    let buf = BufReader::new(server);
    let mut stream = utils::receive(buf);

    while let Some(msg) = stream.next().await {
        match msg? {
            Server::Message {chat_name,message} => {
                println!("In chat {}: {}",chat_name, message);
            }
            Server::Error(message)=>{
                println!("Error recieved: {}",message);
            }
        }
    }
    Ok(())


}

fn main() -> ChatResult<()> {

    let addr = std::env::args().nth(1).expect("Address:PORT");
    task::block_on(async {
        let socket = net::TcpStream::connect(addr).await?;
        socket.set_nodelay(true)?;
        let send = send(socket.clone());
        let replies = messages(socket);
        replies.race(send).await?;

        Ok(())
    })


}