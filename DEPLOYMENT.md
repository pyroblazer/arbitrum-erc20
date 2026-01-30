# Deployment Guide

## Quick Start Deployment

### Step 1: Environment Setup

Create a `.env` file in your project root:

```bash
# .env
PRIVATE_KEY=your_private_key_here
RPC_URL=https://sepolia-rollup.arbitrum.io/rpc
```

### Step 2: Build the Contract

```bash
# Build with optimizations
cargo build --release --target wasm32-unknown-unknown

# Verify the build
ls target/wasm32-unknown-unknown/release/*.wasm
```

### Step 3: Deploy to Arbitrum

```bash
# Deploy to Arbitrum Sepolia (testnet)
cargo stylus deploy \
  --private-key-path=<(echo $PRIVATE_KEY) \
  --endpoint=$RPC_URL

# Save the deployed contract address
export TOKEN_ADDRESS=<deployed_address>
```

### Step 4: Initialize the Token

After deployment, you must initialize the token. Create a script `initialize.js`:

```javascript
// initialize.js
const { ethers } = require('ethers');
require('dotenv').config();

const TOKEN_ADDRESS = process.env.TOKEN_ADDRESS;
const PRIVATE_KEY = process.env.PRIVATE_KEY;
const RPC_URL = process.env.RPC_URL;

// ERC-20 ABI (you'll need to export this from your contract)
const ERC20_ABI = [
  "function initialize(string memory tokenName, string memory tokenSymbol, uint8 tokenDecimals, uint256 initialSupply, address initialOwner) external",
  "function name() external view returns (string memory)",
  "function symbol() external view returns (string memory)",
  "function decimals() external view returns (uint8)",
  "function totalSupply() external view returns (uint256)",
  "function balanceOf(address owner) external view returns (uint256)"
];

async function main() {
  // Setup provider and signer
  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
  
  // Connect to contract
  const token = new ethers.Contract(TOKEN_ADDRESS, ERC20_ABI, wallet);
  
  console.log("Initializing token...");
  
  // Initialize with parameters
  const tx = await token.initialize(
    "My Token",                           // name
    "MTK",                                // symbol
    18,                                   // decimals
    ethers.parseUnits("1000000", 18),    // 1M tokens
    wallet.address                        // owner
  );
  
  console.log("Transaction hash:", tx.hash);
  console.log("Waiting for confirmation...");
  
  await tx.wait();
  
  console.log("Token initialized successfully!");
  
  // Verify initialization
  const name = await token.name();
  const symbol = await token.symbol();
  const decimals = await token.decimals();
  const totalSupply = await token.totalSupply();
  const balance = await token.balanceOf(wallet.address);
  
  console.log("\n=== Token Details ===");
  console.log("Name:", name);
  console.log("Symbol:", symbol);
  console.log("Decimals:", decimals);
  console.log("Total Supply:", ethers.formatUnits(totalSupply, decimals));
  console.log("Owner Balance:", ethers.formatUnits(balance, decimals));
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
```

Run the initialization:
```bash
node initialize.js
```

## Production Deployment Checklist

### Pre-Deployment

- [ ] Code review completed
- [ ] Security audit performed (recommended for mainnet)
- [ ] All tests passing (`cargo test`)
- [ ] Test deployment on testnet
- [ ] Token parameters decided:
  - [ ] Name
  - [ ] Symbol
  - [ ] Decimals (typically 18)
  - [ ] Initial supply
  - [ ] Owner address
- [ ] Access control strategy determined
- [ ] Emergency procedures documented

### Deployment

- [ ] Build with release profile
- [ ] Verify WASM binary size is reasonable
- [ ] Deploy to testnet first
- [ ] Test all functions on testnet
- [ ] Deploy to mainnet
- [ ] Initialize token immediately after deployment
- [ ] Verify contract on block explorer

### Post-Deployment

- [ ] Save all deployment details:
  - [ ] Contract address
  - [ ] Deployment transaction hash
  - [ ] Initialization transaction hash
  - [ ] Block number
  - [ ] Timestamp
- [ ] Transfer initial tokens if needed
- [ ] Update frontend/documentation with contract address
- [ ] Announce deployment to users
- [ ] Monitor initial transactions
- [ ] Set up alerts for unusual activity

## Configuration Examples

### Example 1: Fixed Supply Token (Recommended for simplicity)

```rust
// In your deployment script
token.initialize(
    "Fixed Token",
    "FIX",
    18,
    U256::from(1_000_000_000_000_000_000_000_000), // 1M fixed
    owner_address
);

// After initialization, optionally renounce ownership
// to make it truly fixed and decentralized
token.renounce_ownership();
```

**Use Case:** Governance tokens, community tokens, fair launch tokens

### Example 2: Mintable Supply Token

```rust
// In your deployment script
token.initialize(
    "Mintable Token",
    "MINT",
    18,
    U256::from(100_000_000_000_000_000_000_000), // 100K initial
    multisig_address // Use multi-sig for security
);

// Keep ownership for minting
// DO NOT renounce
```

**Use Case:** Rewards tokens, inflationary tokens, ecosystem tokens

### Example 3: Stablecoin-like Token

```rust
// In your deployment script
token.initialize(
    "My Stablecoin",
    "MSC",
    6, // USDC uses 6 decimals
    U256::from(0), // Start with 0 supply
    treasury_multisig_address
);

// Mint as needed based on reserves
// Implement strict minting controls
```

**Use Case:** Stablecoins, backed tokens, synthetic assets

### Example 4: Gaming/App Token

```rust
// In your deployment script
token.initialize(
    "Game Token",
    "GAME",
    18,
    U256::from(1_000_000_000_000_000_000_000_000_000), // 1B tokens
    game_contract_address
);

// Game contract handles distribution
// Consider implementing burn mechanisms for deflationary pressure
```

**Use Case:** In-game currencies, app tokens, utility tokens

## Network-Specific Deployment

### Arbitrum Sepolia (Testnet)
```bash
RPC_URL=https://sepolia-rollup.arbitrum.io/rpc
CHAIN_ID=421614
EXPLORER=https://sepolia.arbiscan.io
```

### Arbitrum One (Mainnet)
```bash
RPC_URL=https://arb1.arbitrum.io/rpc
CHAIN_ID=42161
EXPLORER=https://arbiscan.io
```

### Arbitrum Nova
```bash
RPC_URL=https://nova.arbitrum.io/rpc
CHAIN_ID=42170
EXPLORER=https://nova.arbiscan.io
```

## Gas Optimization Tips

1. **Batch Operations**: Group multiple transactions together
2. **Use Events**: Events are cheaper than storing data
3. **Minimize Storage Writes**: Only write when necessary
4. **Zero Amount Checks**: Already optimized in implementation
5. **Allowance Patterns**: Use `increaseAllowance` instead of `approve(0)` then `approve(X)`

## Security Best Practices

### Owner Key Management

```bash
# NEVER commit private keys to git
echo ".env" >> .gitignore

# Use hardware wallet for mainnet owner
# Examples:
# - Ledger
# - Trezor
# - Gnosis Safe (multi-sig)

# For production, use multi-sig as owner
MULTISIG_ADDRESS=0x...  # 3-of-5 multi-sig recommended
```

### Monitoring Setup

```javascript
// monitor.js - Example monitoring script
const { ethers } = require('ethers');

const TOKEN_ADDRESS = process.env.TOKEN_ADDRESS;
const RPC_URL = process.env.RPC_URL;

const ERC20_ABI = [/* your ABI */];

async function monitor() {
  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const token = new ethers.Contract(TOKEN_ADDRESS, ERC20_ABI, provider);
  
  // Monitor Transfer events
  token.on("Transfer", (from, to, amount, event) => {
    console.log(`Transfer: ${from} -> ${to}: ${ethers.formatUnits(amount, 18)}`);
    
    // Alert on large transfers
    if (ethers.parseUnits(amount, 18) > ethers.parseUnits("10000", 18)) {
      console.warn("⚠️ LARGE TRANSFER DETECTED!");
      // Send alert to monitoring service
    }
  });
  
  // Monitor Pause events
  token.on("Paused", (account) => {
    console.error("🚨 TOKEN PAUSED by", account);
    // Send urgent alert
  });
  
  // Monitor ownership changes
  token.on("OwnershipTransferred", (previousOwner, newOwner) => {
    console.warn("⚠️ OWNERSHIP TRANSFERRED");
    console.log("From:", previousOwner);
    console.log("To:", newOwner);
    // Send alert
  });
  
  console.log("Monitoring token events...");
}

monitor().catch(console.error);
```

## Troubleshooting

### Issue: "Already Initialized" Error
**Solution**: The token can only be initialized once. Deploy a new contract if needed.

### Issue: "Not Owner" Error
**Solution**: The operation requires owner privileges. Check you're using the correct account.

### Issue: Insufficient Gas
**Solution**: Increase gas limit in your transaction. Stylus contracts may require more gas.

### Issue: "Paused" Error
**Solution**: Token is currently paused. Contact owner to unpause or wait.

### Issue: "Insufficient Balance/Allowance"
**Solution**: Ensure sender has enough tokens and proper approvals are set.

## Emergency Procedures

### In Case of Emergency

1. **Pause the Token** (if you're the owner):
   ```javascript
   await token.pause();
   ```

2. **Investigate the Issue**:
   - Check recent transactions
   - Verify contract state
   - Identify the problem

3. **Communicate**:
   - Notify users immediately
   - Explain the situation
   - Provide timeline for resolution

4. **Resolve and Unpause**:
   ```javascript
   await token.unpause();
   ```

### When to Pause

- Detected exploit or vulnerability
- Unusual transaction patterns
- Smart contract bug discovered
- Security incident in progress

### When NOT to Pause

- Normal market volatility
- User complaints about price
- Routine operations
- Minor cosmetic issues

## Verification

After deployment, verify your contract on the block explorer:

```bash
# Export ABI
cargo stylus export-abi > abi.json

# Use block explorer to verify
# Upload source code and ABI
# Example: arbiscan.io -> Verify & Publish Contract Source Code
```

## Mainnet Deployment Final Checklist

Before deploying to mainnet with real value:

- [ ] ✅ Security audit completed
- [ ] ✅ All tests passing
- [ ] ✅ Testnet deployment successful
- [ ] ✅ Token economics reviewed
- [ ] ✅ Owner security measures in place (hardware wallet/multi-sig)
- [ ] ✅ Emergency procedures documented
- [ ] ✅ Monitoring setup complete
- [ ] ✅ Team trained on operations
- [ ] ✅ Legal compliance checked
- [ ] ✅ Community communication plan ready
- [ ] ✅ Sufficient gas funds available
- [ ] ✅ Backup plans prepared

## Support Resources

- [Arbitrum Discord](https://discord.gg/arbitrum)
- [Stylus Documentation](https://docs.arbitrum.io/stylus)
- [Arbitrum Forum](https://forum.arbitrum.foundation/)

---

**Remember**: Deploying a token on mainnet is a serious responsibility. Always test thoroughly and consider professional security audits for production deployments.