const { ethToEvmos, evmosToEth } = require('@evmos/address-converter');

// Load address_manager.json file
const fs = require('fs');
const file_name = "address_manager.json";
const address_manager = JSON.parse(fs.readFileSync(file_name, 'utf8'));

const address_map = address_manager['converted_address_map'];
// Check length of address_map
const num_of_address = Object.keys(address_map).length;
console.log(`Number of addresses: ${num_of_address}`);

// Convert to evmos addresses
const evmos_address_map = {};
Object.entries(address_map).forEach(([key, _]) => {
    const evmos_address = ethToEvmos(key);
    evmos_address_map[key] = evmos_address;
});

// Save addresses map to json file
fs.writeFileSync(`hex_to_evmos_addresses_map_${num_of_address}.json`, JSON.stringify(evmos_address_map, null, 2));

console.log('Successfully converted addresses');
