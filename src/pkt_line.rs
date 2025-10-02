use bytes::BytesMut;

pub fn write_pkt_line(data: String) -> BytesMut {
    let lens = data.len();
    let header = format!("{:04x}", lens + 5); // self + \n
    let data = format!("{}{}\n", header, data);
    BytesMut::from(data.as_str())
}

