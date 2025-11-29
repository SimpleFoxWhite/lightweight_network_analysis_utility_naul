mod checker;
use std::env;
use std::net::{Ipv4Addr};
use std::str::FromStr;
use std::time::Duration;

use checker::{analyze_interfaces, NetworkScanner};


fn main() 
{
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    
    while i < args.len() 
    {
        match args[i].as_str() 
        {
            "-h" | "--help" => 
            {
                println!("Использование: {} [OPTIONS]", args[0]);
                println!("Options:");
                println!("  -h,  --help                                Показать эту справку");
                println!("  -v,  --version                             Показать версию");
                println!("  -i,  --interfaces                          Показать локальные интерфейсы");
                println!("  -an, --analyze_network (параметр)/(маска)  Полный анализ сети");
                println!("  -a,  --analyze                             Анализировать сеть");
                return;
            }

            "-v" | "--version" => 
            {
                println!("Программа версии 1.0.0");
                return;
            }

            "-i" | "--interfaces" => 
            {
                analyze_interfaces();
                return;
            }

            "-an" | "--analyze_network" => 
            {
                // Проверяем, есть ли следующий аргумент (параметр)
                if i + 1 < args.len() 
                {
                    let target = &args[i+1];
                    println!("Анализ сети: {}", target);

                    // Ищем позицию разделителя '/'
                    if let Some(slash_pos) = target.find('/') 
                    {
                        let ip = &target[0..slash_pos];
                        let pref = &target[slash_pos + 1..]; // +1 чтобы пропусить '/'
                        
                        // Дополнительная валидация
                        if let Ok(prefix_num) = pref.parse::<u8>() 
                        {
                            if prefix_num > 31 
                            {
                                println!("Ошибка: маска сети должна быть от 0 до 31");
                            } 
                            else 
                            {
                                match (Ipv4Addr::from_str(ip), pref.parse::<u8>()) {
                                    (Ok(current_ip), Ok(current_pref)) if current_pref <= 32 => {
                                        let mut network = NetworkScanner::new();
                                        network.timeout = Duration::from_secs(5); // исправлено здесь
                                        network.max_threads = 5;
                                        
                                        network.comprehensive_scan(current_ip, current_pref);
                                    }
                                    (Err(e), _) => println!("Ошибка парсинга IP: {}", e),
                                    (_, Err(e)) => println!("Ошибка парсинга маски: {}", e),
                                    (Ok(_), Ok(pref)) => println!("Неверная маска: {}", pref),
                                }
                            }
                        } 
                        else 
                        {
                            println!("Ошибка: некорректная маска сети");
                        }
                    } 
                    else 
                    {
                        println!("Ошибка: неверный формат. Ожидается: IP/маска");
                    }

                    i += 1;
                } 
                else 
                {
                    eprintln!("Ошибка: для {} требуется параметр", args[i]);
                    return;
                }
            }

            "-a" | "--analysing" =>
            {
                return;
            }

            _ => 
            {
                println!("Неизвестный аргумент: {}", args[i]);
                return;
            }
        }
        i+=1;
    }
}