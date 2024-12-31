#[tokio::main]
async fn main(){
    println!("Starting");
    let wp = webplotter::start();
    loop{
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        wp.send("Hello".to_string());
        wp.send("World".to_string());
    }
    

    println!("Started");
    std::thread::park();
    println!("Exiting");
}