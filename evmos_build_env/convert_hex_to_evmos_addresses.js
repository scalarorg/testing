const { ethToEvmos, evmosToEth } = require('@evmos/address-converter');

// Load address_manager.json file
const fs = require('fs');
const address_manager = JSON.parse(fs.readFileSync('address_manager.json', 'utf8'));

const address_map = address_manager['converted_address_map'];

// Convert to evmos addresses
const evmos_address_map = {};
Object.entries(address_map).forEach(([key, _]) => {
    const evmos_address = ethToEvmos(key);
    evmos_address_map[key] = evmos_address;
});

// Save addresses map to json file
fs.writeFileSync('hex_to_evmos_addresses_map.json', JSON.stringify(evmos_address_map, null, 2));

