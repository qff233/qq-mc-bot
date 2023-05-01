use std::net::SocketAddr;

use proc_qq::re_exports::ricq::client::event::GroupMessageEvent;
use proc_qq::{
    event, module, LoginEvent, MessageChainParseTrait, MessageSendToSourceTrait, Module,
};

use dns_lookup::lookup_host;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[event]
async fn login(event: &LoginEvent) -> anyhow::Result<bool> {
    tracing::info!("正在登录 : {}", event.uin);
    Ok(false)
}

#[event(bot_command = "/ping")]
async fn ping(event: &GroupMessageEvent) -> anyhow::Result<bool> {
    event
        .send_message_to_source("hi~".parse_message_chain())
        .await?;
    Ok(true)
}

fn create_packet(packet_id: u32, data: &Vec<u8>) -> Vec<u8> {
    let pid = to_var_int(packet_id);
    let length = to_var_int((data.len() + pid.len()) as u32);

    let mut buf = length;
    for i in pid {
        buf.push(i);
    }

    for &i in data {
        buf.push(i);
    }

    return buf;
}

const SEGMENT_BITS: u8 = 0x7f;
const CONTINUE_BIT: u8 = 0x80;
async fn read_var_int(reader: &mut BufReader<&mut TcpStream>) -> Result<i32, std::io::Error> {
    let mut value: i32 = 0;
    let mut position: u32 = 0;
    let mut current_byte: u8;

    loop {
        current_byte = reader.read_u8().await?;

        value |= ((current_byte & SEGMENT_BITS).wrapping_shl(position)) as i32;

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }

        position += 7;
        if position > 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "position error",
            ));
        }
    }

    return Ok(value);
}

async fn read_string(reader: &mut BufReader<&mut TcpStream>) -> Result<String, std::io::Error> {
    let length = read_var_int(reader).await?;

    let mut result: Vec<u8> = Vec::new();
    result.resize(length as usize, 0);
    reader.read_exact(&mut result).await?;
    Ok(String::from_utf8(result).unwrap())
}

fn to_var_int(mut value: u32) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    loop {
        if (value as u8 & !SEGMENT_BITS) == 0 {
            result.push(value as u8);
            return result;
        }

        result.push((value as u8 & SEGMENT_BITS) | CONTINUE_BIT);
        value >>= 7;
    }
}

async fn api_mcping(host: &str) -> String {
    let host_port: Vec<&str> = host.split(':').collect();
    let port = match host_port.get(1) {
        Some(&port) => match port.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                tracing::info!("port erro.");
                return String::from("port error.");
            }
        },
        None => 25565,
    };

    let ip = match lookup_host(host_port[0]) {
        Ok(ips) => {
            let mut ret_ip = ips[0];
            for ip in ips {
                if ip.is_ipv4() {
                    ret_ip = ip;
                    break;
                }
            }
            ret_ip
        }
        Err(_) => {
            tracing::info!("lookup host error.");
            return String::from("lookup host error.");
        }
    };

    let socket_addr = SocketAddr::new(ip, port);
    let mut stream = match TcpStream::connect(&socket_addr).await {
        Ok(stream) => stream,
        Err(_) => {
            tracing::info!("can't connect the server");
            return String::from("connet error.");
        }
    };

    let mut buffer: Vec<u8> = Vec::new();
    let mut add_buf = |buf: &[u8]| -> () {
        for &i in buf {
            buffer.push(i);
        }
    };
    let port_buf: Vec<u8> = vec![(port >> 8) as u8, (port & 0xFF) as u8];
    add_buf(&to_var_int(u32::MAX));
    add_buf(&to_var_int(host.len() as u32));
    add_buf(host_port[0].as_bytes());
    add_buf(&port_buf);
    add_buf(&to_var_int(1));

    if let Err(_) = stream.write(&create_packet(0x00, &buffer)).await {
        tracing::info!("send packet error!");
        return String::from("send packet error!");
    }

    if let Err(_) = stream.write(&create_packet(0x00, &Vec::new())).await {
        tracing::info!("send packet error!");
        return String::from("send packet error!");
    }

    let mut reader = BufReader::new(&mut stream);
    let _length = match read_var_int(&mut reader).await {
        Ok(len) => len,
        Err(e) => {
            tracing::info!("recv packet error!");
            return String::from(format!("recv packet error! {}", e.to_string()));
        }
    };
    let _pocket_id = match read_var_int(&mut reader).await {
        Ok(id) => id,
        Err(e) => {
            tracing::info!("recv packet error!");
            return String::from(format!("recv packet error! {}", e.to_string()));
        }
    };

    let data = match read_string(&mut reader).await {
        Ok(val) => val,
        Err(e) => {
            tracing::info!("recv packet error!");
            return String::from(format!("recv packet error! {}", e.to_string()));
        }
    };
    data
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_packet_test() {
        let data: Vec<u8> = vec![0x11];
        let buf = create_packet(0x00, &data);

        assert_eq!(buf, vec![2, 0, 17]);
        assert_eq!(to_var_int(4294967295), vec![255, 255, 255, 255, 15]);
    }

    #[test]
    fn recv_buf() {
        let buf;
        buf = tokio_test::block_on(api_mcping("3f.z4cs.com"));
        panic!("{}", buf);
    }
}

#[event(bot_command = "/mcping {host}")]
async fn mc_ping(event: &GroupMessageEvent, host: String) -> anyhow::Result<bool> {
    tracing::info!("recv {}", host);
    let data = api_mcping(host.as_str()).await;
    event
        .send_message_to_source(data.parse_message_chain())
        .await?;
    Ok(true)
}

pub fn module() -> Module {
    module!("ping", "ping", ping, mc_ping)
}
