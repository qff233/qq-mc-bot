use base64::{engine::general_purpose, Engine as _};
use dns_lookup::lookup_host;
use json;
use proc_qq::{
    event, module, LoginEvent, MessageChainParseTrait, MessageSendToSourceTrait, Module,
};
use proc_qq::{re_exports::ricq::client::event::GroupMessageEvent, MessageChainAppendTrait};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub fn module() -> Module {
    module!("ping", "ping", login, ping, mc_ping)
}

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

const SEGMENT_BITS: u32 = 0x7F;
const CONTINUE_BIT: u32 = 0x80;
async fn read_var_int(reader: &mut BufReader<&mut TcpStream>) -> Result<i32, std::io::Error> {
    let mut value: i32 = 0;
    let mut position: u32 = 0;
    let mut current_byte: u32;

    loop {
        current_byte = reader.read_u8().await? as u32;

        value |= ((current_byte & SEGMENT_BITS) << position) as i32;

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
    let length = read_var_int(reader).await? as usize;

    let mut result: Vec<u8> = Vec::new();
    result.resize(length, 0);
    reader.read_exact(&mut result).await?;
    Ok(String::from_utf8(result).unwrap())
}

fn to_var_int(mut value: u32) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    loop {
        if (value & !SEGMENT_BITS) == 0 {
            result.push(value as u8);
            return result;
        }

        result.push(((value & SEGMENT_BITS) | CONTINUE_BIT) as u8);
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
    // #[should_panic]
    fn recv_buf() {
        let buf;
        buf = tokio_test::block_on(api_mcping("3f.z4cs.com"));
        panic!("{}", buf);
    }

    #[test]
    fn base64_test() {
        let base = r"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";
        let result = general_purpose::STANDARD.decode(base).unwrap();
        panic!("{:?}", result);
    }
}

#[event(bot_command = "/mcping {host}")]
async fn mc_ping(event: &GroupMessageEvent, host: String) -> anyhow::Result<bool> {
    tracing::info!("recv {}", host);
    let data = api_mcping(host.as_str()).await;
    let mut result = String::new();
    if let Some(str) = data.get(..1) {
        if str == "{" {
            let json_result = json::parse(&data).unwrap();
            let description = json_result["description"].to_string();
            if !description.is_empty() {
                result += format!("服务器介绍：{}\n", description).as_str();
            }

            let players = &json_result["players"];
            let players_max = players["max"].to_string();
            let players_online = players["online"].to_string();
            let players_sample = &players["sample"];
            let mut samples: Vec<(String, String)> = Vec::new();
            for i in 0.. {
                if players_sample[i].is_empty() {
                    break;
                }
                let sample = &players_sample[i];
                samples.push((sample["id"].to_string(), sample["name"].to_string()));
            }
            if !players_max.is_empty() && !players_online.is_empty() {
                result += format!("玩家在线人数：{}/{}\n", players_online, players_max).as_str();
            }
            if !samples.is_empty() {
                result += "玩家列表：\n";
                for i in samples {
                    result += format!("  {}\n", i.1).as_str();
                }
            }

            let version = &json_result["version"];
            let version_name = version["name"].to_string();
            if !version_name.is_empty() {
                result += format!("服务器版本：{}\n", version_name).as_str();
            }

            let mut img: Option<Vec<u8>> = None;
            let favicon = json_result["favicon"].to_string();
            if !favicon.is_empty() {
                let favicon = favicon.replace("\n", "");
                let index = favicon.find(",").unwrap();
                let favicon_base64 = &favicon[(index + 1)..];
                // panic!("{}", favicon_base64);
                let raw = general_purpose::STANDARD.decode(favicon_base64).unwrap();
                img = Some(raw);
            }

            match img {
                Some(img) => {
                    let img = event.upload_image_to_source(img).await?;
                    event
                        .send_message_to_source(result.parse_message_chain().append(img))
                        .await?;
                }
                None => {
                    event
                        .send_message_to_source(result.parse_message_chain())
                        .await?;
                }
            }
        } else {
            event
                .send_message_to_source(data.parse_message_chain())
                .await?;
        }
    }
    Ok(true)
}
