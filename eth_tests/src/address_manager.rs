use ethers::{
    prelude::*,
    types::H160,
    signers::coins_bip39::English,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use serde::{Serialize, Deserialize};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::signers::Wallet;
use rustc_hex::ToHex;

#[derive(Serialize, Deserialize, Clone)]
pub struct AddressInfo {
    pub address: H160,
    pub private_key: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddressManager {
    mnemonic: String,
    converted_addresses: Vec<AddressInfo>,
    unconverted_address_map: HashMap<H160, usize>,
    converted_address_map: HashMap<H160, usize>,
}

impl AddressManager {
    pub fn new(mnemonic: &str) -> Self {
        AddressManager {
            mnemonic: mnemonic.to_string(),
            converted_addresses: Vec::new(),
            unconverted_address_map: HashMap::new(),
            converted_address_map: HashMap::new(),
        }
    }

    pub fn info(&self) -> (String, usize) {
        println!("Mnemonic: {}", self.mnemonic);
        println!("Number of allocated addresses: {}", self.converted_address_map.len());
        (self.mnemonic.clone(), self.converted_address_map.len())
    }

    pub fn get_map_address(&mut self, address: &H160) -> Option<H160> {
        if let Some(&index) = self.unconverted_address_map.get(address) {
            return Some(self.converted_addresses[index].address);
        }
    
        let index = self.converted_addresses.len();
        match self.generate_address(index){
            Ok(addr_info) => {
                self.converted_addresses.push(addr_info.clone());
                self.unconverted_address_map.insert(*address, index);
                self.converted_address_map.insert(addr_info.address, index);
                Some(addr_info.address)
            }
            Err(err) => {
                println!("Error generating address: {:?}", err);
                None
            }
        }
    }

    pub fn get_index_with_converted_address(&self, address: &H160) -> Option<usize> {
        self.converted_address_map.get(address).map(|x| *x)
    }

    pub fn get_converted_address_with_index(&self, index: usize) -> String {
        let converted_address = self.converted_addresses.get(index).map(|x| x.address).unwrap();
        format!("0x{}", hex::encode(converted_address))
    }

    pub fn generate_address(&self, index: usize) -> Result<AddressInfo, Box<dyn std::error::Error>> {
        let mnemonic = self.mnemonic.as_ref();
        let wallet: Wallet<SigningKey> = MnemonicBuilder::<English>::default()
            .phrase(mnemonic)
            .index(index as u32)?
            .build()?;

        let address = wallet.address();
        let private_key = wallet.signer().to_bytes().to_hex();
        let public_key = wallet.signer().verifying_key().to_encoded_point(false).as_bytes().to_hex();

        Ok(AddressInfo {
            address,
            private_key,
            public_key,
        })
    }
    
    pub fn get_private_key(&self, address: &H160) -> Option<&str> {
        if let Some(index) = self.get_index_with_converted_address(address) {
            return Some(&self.converted_addresses[index].private_key);
        }
        None
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        let manager: AddressManager = serde_json::from_str(&json)?;
        Ok(manager)
    }
}
