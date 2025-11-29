use pnet_datalink::{interfaces, NetworkInterface};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, TcpStream, SocketAddr};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread;

use pnet::datalink::{self, Channel};
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;


pub fn analyze_interfaces() 
{
    for iface in interfaces() 
    {
        println!("Название интерфейса: {}", iface.name);
        println!("Описание: {}", iface.description);
        println!("Индекс: {}", iface.index);
        println!("Мак адрес: {}", iface.mac.unwrap_or(pnet_datalink::MacAddr::zero()));
        for ip in &iface.ips
        {
            println!("IP: {}/{}", ip.ip(), ip.prefix());
        }

        let iface_type =
        if iface.is_loopback() 
        {
            "Loopback"
        } 
        else if iface.is_up() 
        {
            "Активный"
        } 
        else 
        {
            "Выключен"
        };
        println!("Статус: {}", iface_type);
        println!("===================================")
    }
}

pub fn get_default_interface() -> Option<NetworkInterface> 
{
    interfaces()
        .into_iter()
        .find(|iface| 
        {
            iface.is_up() && 
            !iface.is_loopback() && 
            iface.ips.iter().any(|ip| ip.is_ipv4())
        })
}

pub fn get_local_network() -> Option<(Ipv4Addr, u8)> 
{
    let iface = get_default_interface()?;
    
    for ip_net in iface.ips 
    {
        if let IpAddr::V4(ip) = ip_net.ip() 
        {
            if !ip.is_loopback() && !ip.is_link_local() 
            {
                return Some((ip, ip_net.prefix()));
            }
        }
    }
    None
}

//######################################################################

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub ip: Ipv4Addr,
    pub mac: Option<String>,
    pub hostname: Option<String>,
    pub open_ports: Vec<u16>,
    pub services: Vec<String>,
}

pub struct NetworkScanner {
    pub timeout: Duration,
    pub max_threads: usize,
}

impl NetworkScanner {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_millis(1000),
            max_threads: 100,
        }
    }
    
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn comprehensive_scan(&self, network: Ipv4Addr, prefix: u8) -> Vec<DeviceInfo> {
        println!("🎯 Начинаем сканирование сети {}/{}", network, prefix);
        
        let hosts = self.generate_hosts(network, prefix);
        let devices = Arc::new(Mutex::new(Vec::new()));
        let chunk_size = (hosts.len() / self.max_threads).max(1);
        
        let host_chunks: Vec<Vec<Ipv4Addr>> = hosts
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        let mut handles = vec![];
        
        for chunk in host_chunks {
            let devices = Arc::clone(&devices);
            let timeout = self.timeout;
            
            let handle = thread::spawn(move || {
                for host in chunk {
                    if let Some(device) = Self::scan_host(host, timeout) {
                        let mut devices = devices.lock().unwrap();
                        devices.push(device);
                    }
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let result = devices.lock().unwrap().clone();
        println!("📊 Найдено устройств: {}", result.len());
        result
    }
    
    fn generate_hosts(&self, network: Ipv4Addr, prefix: u8) -> Vec<Ipv4Addr> {
        let mut hosts = Vec::new();
        let base_octets = network.octets();
        let host_bits = 32 - prefix;
        let host_count = 2u32.pow(host_bits as u32) - 2;
        
        if prefix >= 24 {
            for i in 1..=254 {
                if i != base_octets[3] {
                    hosts.push(Ipv4Addr::new(base_octets[0], base_octets[1], base_octets[2], i));
                }
            }
        } else {
            for i in 1..=host_count.min(1000) {
                let ip = self.calculate_ip(network, prefix, i);
                hosts.push(ip);
            }
        }
        
        hosts
    }
    
    fn calculate_ip(&self, network: Ipv4Addr, prefix: u8, host: u32) -> Ipv4Addr {
        let network_int = u32::from(network);
        let host_int = network_int | host;
        
        Ipv4Addr::new(
            ((host_int >> 24) & 0xFF) as u8,
            ((host_int >> 16) & 0xFF) as u8,
            ((host_int >> 8) & 0xFF) as u8,
            (host_int & 0xFF) as u8,
        )
    }
    
    fn scan_host(ip: Ipv4Addr, timeout: Duration) -> Option<DeviceInfo> {
        if !Self::is_host_alive(ip, timeout) {
            return None;
        }
        
        println!("🔍 Анализируем устройство: {}", ip);
        
        let mut device = DeviceInfo {
            ip,
            mac: Self::get_mac_address(ip),
            hostname: Self::get_hostname(ip),
            open_ports: Self::scan_ports(ip, timeout),
            services: Vec::new(),
        };
        
        device.services = Self::identify_services(&device.open_ports);
        
        Some(device)
    }
    
    fn is_host_alive(ip: Ipv4Addr, timeout: Duration) -> bool {
        if TcpStream::connect_timeout(&SocketAddr::from((ip, 80)), timeout).is_ok() {
            return true;
        }
        let ports = [22, 135, 443, 3389];
        for &port in &ports {
            if TcpStream::connect_timeout(&SocketAddr::from((ip, port)), timeout).is_ok() {
                return true;
            }
        }
        
        false
    }
    
    fn get_mac_address(_ip: Ipv4Addr) -> Option<String> {
        None
    }
    
    fn get_hostname(ip: Ipv4Addr) -> Option<String> {
        match dns_lookup::lookup_addr(&std::net::IpAddr::V4(ip)) {
            Ok(hostname) => Some(hostname),
            Err(_) => None,
        }
    }
    
    fn scan_ports(ip: Ipv4Addr, timeout: Duration) -> Vec<u16> {
        let common_ports = [
            21, 22, 23, 25, 53, 80, 110, 135, 139, 143, 443, 445, 
            993, 995, 1723, 3306, 3389, 5900, 8080
        ];
        
        let mut open_ports = Vec::new();
        
        for &port in &common_ports {
            if TcpStream::connect_timeout(&SocketAddr::from((ip, port)), timeout).is_ok() {
                open_ports.push(port);
                println!("   ✅ Порт {} открыт", port);
            }
        }
        
        open_ports
    }
    
    fn identify_services(ports: &[u16]) -> Vec<String> {
        let service_map: HashMap<u16, &str> = [
            (21, "FTP"), (22, "SSH"), (23, "Telnet"), (25, "SMTP"),
            (53, "DNS"), (80, "HTTP"), (110, "POP3"), (135, "RPC"),
            (139, "NetBIOS"), (143, "IMAP"), (443, "HTTPS"), 
            (445, "SMB"), (993, "IMAPS"), (995, "POP3S"),
            (1723, "PPTP"), (3306, "MySQL"), (3389, "RDP"),
            (5900, "VNC"), (8080, "HTTP-Proxy")
        ].iter().cloned().collect();
        
        ports.iter()
            .filter_map(|port| service_map.get(port).map(|s| s.to_string()))
            .collect()
    }
}

//########################################################################################3

pub struct TrafficAnalyzer {
    stats: HashMap<String, usize>,
}

impl TrafficAnalyzer {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }
    
    pub fn start_sniffing(&mut self, interface_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let interface = datalink::interfaces()
            .into_iter()
            .find(|iface| iface.name == interface_name)
            .ok_or("Интерфейс не найден")?;
        
        let (_tx, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err("Непподерживаемый тип соединения".into()),
            Err(e) => return Err(e.into()),
        };
        
        println!("👂 Начинаем прослушивание на интерфейсе: {}", interface_name);
        
        loop {
            match rx.next() {
                Ok(packet) => {
                    self.process_packet(&packet);
                }
                Err(e) => {
                    eprintln!("Ошибка при чтении пакета: {}", e);
                }
            }
        }
    }
    
    fn process_packet(&mut self, packet: &[u8]) {
        if let Some(ethernet) = EthernetPacket::new(packet) {
            match ethernet.get_ethertype() {
                EtherTypes::Ipv4 => {
                    if let Some(ipv4) = Ipv4Packet::new(ethernet.payload()) {
                        self.analyze_ip_packet(&ipv4);
                    }
                }
                _ => {}
            }
        }
    }
    
    fn analyze_ip_packet(&mut self, ip_packet: &Ipv4Packet) {
        let src = ip_packet.get_source();
        let dst = ip_packet.get_destination();
        let protocol = ip_packet.get_next_level_protocol();
        
        // Собираем статистику
        *self.stats.entry(format!("{:?}", protocol)).or_insert(0) += 1;
        
        match protocol {
            IpNextHeaderProtocols::Tcp => {
                if let Some(tcp) = TcpPacket::new(ip_packet.payload()) {
                    println!("📨 TCP: {}:{} -> {}:{} [{} bytes]", 
                             src, tcp.get_source(), 
                             dst, tcp.get_destination(),
                             ip_packet.get_total_length());
                }
            }
            IpNextHeaderProtocols::Udp => {
                if let Some(udp) = UdpPacket::new(ip_packet.payload()) {
                    println!("📨 UDP: {}:{} -> {}:{} [{} bytes]",
                             src, udp.get_source(),
                             dst, udp.get_destination(),
                             ip_packet.get_total_length());
                }
            }
            IpNextHeaderProtocols::Icmp => {
                println!("📨 ICMP: {} -> {}", src, dst);
            }
            _ => {}
        }
    }
    
    pub fn print_stats(&self) {
        println!("\n=== СТАТИСТИКА ТРАФИКА ===");
        for (protocol, count) in &self.stats {
            println!("{}: {} пакетов", protocol, count);
        }
    }
}

//###################################################################


pub fn analyze_network() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 КОМПЛЕКСНЫЙ АНАЛИЗ ЛОКАЛЬНОЙ СЕТИ");
    println!("====================================\n");

    analyze_interfaces();
    

    if let Some((local_ip, prefix)) = get_local_network() {
        println!("\n📍 Локальная сеть: {}/{}", local_ip, prefix);
        

        let scanner = NetworkScanner::new()
            .set_timeout(Duration::from_millis(500));
        
        let devices = scanner.comprehensive_scan(local_ip, prefix);
        

        println!("\n=== ОБНАРУЖЕННЫЕ УСТРОЙСТВА ===");
        for device in devices {
            println!("\n🖥️  Устройство: {}", device.ip);
            if let Some(hostname) = device.hostname {
                println!("   Название: {}", hostname);
            }
            if let Some(mac) = device.mac {
                println!("   MAC: {}", mac);
            }
            println!("   Открытые порты: {:?}", device.open_ports);
            println!("   Службы: {:?}", device.services);
        }
        
        analyze_routing();
        
    } else {
        println!("❌ Не удалось определить локальную сеть");
    }
    
    Ok(())
}

fn analyze_routing() {
    println!("\n=== ТАБЛИЦА МАРШРУТИЗАЦИИ ===");
    
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("route")
            .arg("print")
            .output()
            .expect("Failed to execute route command");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("ip")
            .arg("route")
            .output()
            .expect("Failed to execute ip command");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("netstat")
            .arg("-nr")
            .output()
            .expect("Failed to execute netstat command");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
}