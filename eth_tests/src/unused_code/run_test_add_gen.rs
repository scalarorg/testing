use eth_tests::address_manager::AddressManager;
use ethers::types::H160;

const DEFAULT_MNEMONIC: &str = "orphan assume tent east october arena empower light notice border scatter off thought hawk link lion coconut void huge elegant crucial decline adjust pride";
const TEST_ACCOUNT: &str = "0xf73FfEC0CAEDDCC755C0CE2411569208444F5ca1";

#[tokio::main]
async fn main() {
    let mnemonic = DEFAULT_MNEMONIC;
    let mut address_manager = AddressManager::new(mnemonic);

    for i in 10..15 {
        let address = format!("0xf73FfEC0CAEDDCC755C0CE2411569208444F5ca{}", i%10);
        println!("raw: {:?}", address);
        let address: H160 = address.parse().unwrap();
        println!("{:?}", address_manager.get_map_address(&address).await.unwrap());
        println!("{:?}", address_manager.get_map_address(&address).await.unwrap());
    }
}
        
