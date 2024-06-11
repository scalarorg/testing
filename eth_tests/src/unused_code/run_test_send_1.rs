use eth_tests::test;

#[tokio::main]
async fn main(){
    let result = test::test_send_raw_transaction().await;
    println!("{:?}", result);
    let result = test::get_balance().await;
}