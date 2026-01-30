# Deployment Guide

## Overview

This guide provides comprehensive instructions for deploying the Production-Grade ERC-20 token to Arbitrum networks, including testnet and mainnet deployments. It covers environment setup, configuration, deployment scripts, and post-deployment verification.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Environment Setup](#environment-setup)
3. [Testnet Deployment](#testnet-deployment)
4. [Mainnet Deployment](#mainnet-deployment)
5. [Configuration Examples](#configuration-examples)
6. [Production Checklist](#production-checklist)
7. [Verification](#verification)
8. [Troubleshooting](#troubleshooting)
9. [Emergency Procedures](#emergency-procedures)

---

## Prerequisites

### Required Software

- **Rust**: 1.70.0 or later
- **Cargo**: Latest stable version
- **Arbitrum Stylus SDK**: Latest version
- **Node.js**: 18.0 or later (for deployment scripts)
- **npm or yarn**: Latest version

### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Stylus SDK
cargo install cargo-stylus

# Verify installation
cargo stylus --version
rustc --version
```

### Wallet Setup

For mainnet deployments, use a hardware wallet or secure multi-sig:

**Hardware Wallets Supported:**
- Ledger (via USB)
- Trezor (via USB)

**Multi-Sig Recommendations:**
- Gnosis Safe (3-of-5 recommended)
- Safe (formerly Gnosis)

---

## Environment Setup

### 1. Create Project Directory

```bash
cd /path/to/your/project
```

### 2. Create .env File

Create a `.env` file in your project root:

```bash
# .env - NEVER commit this file to version control

# Private key (use environment variable or hardware wallet for mainnet)
# For testnet, use a test account with no real funds
PRIVATE_KEY=your_private_key_here

# RPC Endpoints
ARBITRUM_SEPOLIA_RPC=https://sepolia-rollup.arbitrum.io/rpc
ARBITRUM_MAINNET_RPC=https://arb1.arbitrum.io/rpc

# Block Explorer API Keys (optional, for verification)
ARBISCAN_API_KEY=your_arbiscan_api_key
ARBISCAN_API_KEY_MAINNET=your_arbiscan_mainnet_api_key

# Deployment Configuration
DEPLOYMENT_NETWORK=arbitrum-sepolia
CONTRACT_NAME=MyToken
TOKEN_NAME="My Production Token"
TOKEN_SYMBOL=MTK
TOKEN_DECIMALS=18
INITIAL_SUPPLY=1000000
```

### 3. Add .env to .gitignore

```bash
echo ".env" >> .gitignore
echo "*.keystore" >> .gitignore
```

### 4. Configure Cargo.toml

Ensure your `Cargo.toml` has the correct configuration:

```toml
[package]
name = "my-erc20-token"
version = "1.0.0"
edition = "2021"

[dependencies]
stylus-sdk = "0.5.0"
alloy-primitives = "0.3.1"
alloy-sol-types = "0.4.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1

[features]
export-abi = []
```

---

## Testnet Deployment

### Step 1: Get Testnet ETH

1. Go to [Arbitrum Sepolia Faucet](https://sepolia.arbitrum.io/)
2. Request testnet ETH
3. Wait for confirmation (usually instant)

### Step 2: Build the Contract

```bash
# Clean and build with optimizations
cargo clean
cargo build --release --target wasm32-unknown-unknown

# Verify the build
ls -la target/wasm32-unknown-unknown/release/*.wasm

# Expected output: ~20-50KB WASM binary
```

### Step 3: Export ABI

```bash
# Export ABI for frontend integration
cargo stylus export-abi

# Verify ABI file created
ls -la abi.json
```

### Step 4: Deploy to Arbitrum Sepolia

**Method 1: Using Private Key (Development)**

```bash
# Deploy using private key from .env
cargo stylus deploy \
  --private-key-path=<(echo $PRIVATE_KEY) \
  --endpoint=$ARBITRUM_SEPOLIA_RPC \
  --contract-name=$CONTRACT_NAME

# Save the deployed contract address
export TOKEN_ADDRESS=<deployed_address>
echo "Token deployed to: $TOKEN_ADDRESS"
```

**Method 2: Using Environment Variable**

```bash
# Deploy using PRIVATE_KEY from environment
PRIVATE_KEY=$PRIVATE_KEY cargo stylus deploy \
  --endpoint=$ARBITRUM_SEPOLIA_RPC \
  --contract-name=$CONTRACT_NAME
```

**Method 3: Using Hardware Wallet (Recommended)**

```bash
# Deploy using Ledger
cargo stylus deploy \
  --ledger \
  --endpoint=$ARBITRUM_SEPOLIA_RPC \
  --contract-name=$CONTRACT_NAME
```

### Step 5: Initialize the Token

Create an initialization script:

```javascript
// scripts/initialize-testnet.js
const { ethers } = require('ethers');
require('dotenv').config();

// Load configuration
const RPC_URL = process.env.ARBITRUM_SEPOLIA_RPC;
const PRIVATE_KEY = process.env.PRIVATE_KEY;
const TOKEN_ADDRESS = process.env.TOKEN_ADDRESS;

// Token initialization ABI
const INITIALIZE_ABI = [
  "function initialize(string tokenName, string tokenSymbol, uint8 tokenDecimals, uint256 initialSupply, address initialOwner) external"
];

// Read-only ABI for verification
const READ_ABI = [
  "function name() view returns (string)",
  "function symbol() view returns (string)",
  "function decimals() view returns (uint8)",
  "function totalSupply() view returns (uint256)",
  "function balanceOf(address owner) view returns (uint256)",
  "function owner() view returns (address)"
];

async function main() {
  console.log("Initializing token on Arbitrum Sepolia...");
  console.log("Token Address:", TOKEN_ADDRESS);

  // Setup provider and signer
  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

  // Create contract instance
  const token = new ethers.Contract(TOKEN_ADDRESS, INITIALIZE_ABI, wallet);

  // Token parameters
  const tokenName = process.env.TOKEN_NAME || "My Production Token";
  const tokenSymbol = process.env.TOKEN_SYMBOL || "MTK";
  const tokenDecimals = parseInt(process.env.TOKEN_DECIMALS) || 18;
  const initialSupply = ethers.parseUnits(process.env.INITIAL_SUPPLY || "1000000", tokenDecimals);

  console.log("\nInitialization Parameters:");
  console.log("Name:", tokenName);
  console.log("Symbol:", tokenSymbol);
  console.log("Decimals:", tokenDecimals);
  console.log("Initial Supply:", ethers.formatUnits(initialSupply, tokenDecimals));
  console.log("Owner:", wallet.address);

  try {
    // Initialize the token
    const tx = await token.initialize(
      tokenName,
      tokenSymbol,
      tokenDecimals,
      initialSupply,
      wallet.address
    );

    console.log("\nTransaction Hash:", tx.hash);
    console.log("Waiting for confirmation...");

    await tx.wait();

    console.log("\n✅ Token initialized successfully!");

    // Verify initialization
    const readToken = new ethers.Contract(TOKEN_ADDRESS, READ_ABI, provider);
    const [name, symbol, decimals, totalSupply, balance, owner] = await Promise.all([
      readToken.name(),
      readToken.symbol(),
      readToken.decimals(),
      readToken.totalSupply(),
      readToken.balanceOf(wallet.address),
      readToken.owner()
    ]);

    console.log("\n=== Token Verification ===");
    console.log("Name:", name);
    console.log("Symbol:", symbol);
    console.log("Decimals:", decimals);
    console.log("Total Supply:", ethers.formatUnits(totalSupply, decimals));
    console.log("Owner Balance:", ethers.formatUnits(balance, decimals));
    console.log("Owner Address:", owner);

    // Save verification data
    const verificationData = {
      network: "arbitrum-sepolia",
      tokenAddress: TOKEN_ADDRESS,
      deploymentHash: tx.hash,
      initializationHash: tx.hash,
      name,
      symbol,
      decimals,
      totalSupply: totalSupply.toString(),
      owner: wallet.address,
      timestamp: new Date().toISOString()
    };

    console.log("\nVerification data saved.");
    return verificationData;

  } catch (error) {
    console.error("\n❌ Initialization failed:", error.message);
    process.exit(1);
  }
}

main()
  .then((data) => {
    console.log("\n✅ Deployment completed successfully!");
    process.exit(0);
  })
  .catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
  });
```

Run the initialization:

```bash
# Install dependencies
npm init -y
npm install ethers dotenv

# Run initialization
node scripts/initialize-testnet.js
```

### Step 6: Configure Production Features (Optional)

```javascript
// scripts/configure-testnet.js
const { ethers } = require('ethers');
require('dotenv').config();

const RPC_URL = process.env.ARBITRUM_SEPOLIA_RPC;
const PRIVATE_KEY = process.env.PRIVATE_KEY;
const TOKEN_ADDRESS = process.env.TOKEN_ADDRESS;

const CONFIG_ABI = [
  // Supply Cap
  "function setSupplyCap(uint256 newCap) external",
  "function setSupplyCapEnabled(bool enabled) external",
  // Roles
  "function grantRole(uint32 role, address account) external",
  "function hasRole(uint32 role, address account) view returns (bool)",
  // Ownership Time-Lock
  "function setOwnershipTransferDelay(uint256 delaySeconds) external",
  // Emergency
  "function setGuardian(address newGuardian) external"
];

// Role constants
const ADMIN_ROLE = ethers.getUint(ethers.getBytes(ethers.id("ADMIN_ROLE")));
const MINTER_ROLE = ethers.getUint(ethers.getBytes(ethers.id("MINTER_ROLE")));
const PAUSER_ROLE = ethers.getUint(ethers.getBytes(ethers.id("PAUSER_ROLE")));

async function main() {
  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
  const token = new ethers.Contract(TOKEN_ADDRESS, CONFIG_ABI, wallet);

  console.log("Configuring production features...");

  // Example: Set supply cap (1 billion tokens with 18 decimals)
  const supplyCap = ethers.parseUnits("1000000000", 18);
  await token.setSupplyCap(supplyCap);
  await token.setSupplyCapEnabled(true);
  console.log("✅ Supply cap set to 1 billion tokens");

  // Set ownership transfer delay to 48 hours
  const fortyEightHours = 48 * 60 * 60;
  await token.setOwnershipTransferDelay(fortyEightHours);
  console.log("✅ Ownership transfer delay set to 48 hours");

  // Add additional admin (multi-sig example)
  const multisigAddress = "0x..."; // Your multi-sig address
  await token.grantRole(ADMIN_ROLE, multisigAddress);
  console.log("✅ Admin role granted to multi-sig");

  // Set guardian for emergency pause
  const guardianAddress = "0x..."; // Trusted guardian address
  await token.setGuardian(guardianAddress);
  console.log("✅ Guardian set for emergency pause");

  console.log("\n✅ Configuration complete!");
}

main().catch(console.error);
```

### Step 7: Verify on Block Explorer

1. Go to [Arbitrum Sepolia Explorer](https://sepolia.arbiscan.io/)
2. Search for your contract address
3. Click "Verify and Publish" Contract Source Code
4. Upload the source files and ABI
5. Submit for verification

---

## Mainnet Deployment

### ⚠️ Critical Warnings

- **NEVER deploy to mainnet without thorough testnet testing**
- **Use a hardware wallet or multi-sig for deployment**
- **Have emergency procedures ready**
- **Consider professional security audit**
- **Test all functions before mainnet deployment**

### Step 1: Security Checklist

Before proceeding, ensure:

- [ ] All tests passing on testnet
- [ ] Contract verified on testnet explorer
- [ ] All production features tested
- [ ] Security audit completed
- [ ] Bug bounty program active
- [ ] Emergency procedures documented
- [ ] Team trained on operations

### Step 2: Mainnet ETH

Ensure you have sufficient ETH for deployment:

- **Deployment gas**: ~0.1-0.5 ETH depending on gas prices
- **Initialization gas**: ~0.01-0.05 ETH
- **Buffer**: Keep at least 1 ETH for emergencies

### Step 3: Configure for Mainnet

Update your `.env` file:

```bash
# Switch to mainnet RPC
ARBITRUM_MAINNET_RPC=https://arb1.arbitrum.io/rpc
DEPLOYMENT_NETWORK=arbitrum-mainnet

# Use mainnet API key
ARBISCAN_API_KEY=$ARBISCAN_API_KEY_MAINNET
```

### Step 4: Deploy to Mainnet

**Method 1: Hardware Wallet (Recommended)**

```bash
# Deploy using Ledger
cargo stylus deploy \
  --ledger \
  --endpoint=$ARBITRUM_MAINNET_RPC \
  --contract-name=$CONTRACT_NAME
```

**Method 2: Multi-Sig (Most Secure)**

For multi-sig deployment, use Gnosis Safe:

1. Create a deployment transaction
2. Collect required signatures
3. Execute transaction

### Step 5: Initialize on Mainnet

```bash
# Update .env with mainnet values
source .env

# Run initialization script
node scripts/initialize-mainnet.js
```

**Initialization Script (Mainnet):**

```javascript
// scripts/initialize-mainnet.js
const { ethers } = require('ethers');
require('dotenv').config();

const RPC_URL = process.env.ARBITRUM_MAINNET_RPC;
const PRIVATE_KEY = process.env.PRIVATE_KEY;
const TOKEN_ADDRESS = process.env.TOKEN_ADDRESS;

const INITIALIZE_ABI = [
  "function initialize(string tokenName, string tokenSymbol, uint8 tokenDecimals, uint256 initialSupply, address initialOwner) external"
];

async function main() {
  console.log("⚠️  INITIALIZING ON MAINNET");
  console.log("Token Address:", TOKEN_ADDRESS);

  const provider = new ethers.JsonRpcProvider(RPC_URL);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

  if (wallet.address.toLowerCase() !== process.env.OWNER_ADDRESS?.toLowerCase()) {
    console.warn("⚠️  WARNING: Wallet address doesn't match expected owner!");
    console.warn("Expected:", process.env.OWNER_ADDRESS);
    console.warn("Actual:", wallet.address);
  }

  const token = new ethers.Contract(TOKEN_ADDRESS, INITIALIZE_ABI, wallet);

  const tokenName = process.env.TOKEN_NAME;
  const tokenSymbol = process.env.TOKEN_SYMBOL;
  const tokenDecimals = parseInt(process.env.TOKEN_DECIMALS);
  const initialSupply = ethers.parseUnits(process.env.INITIAL_SUPPLY, tokenDecimals);

  console.log("\nInitialization Parameters:");
  console.log("Name:", tokenName);
  console.log("Symbol:", tokenSymbol);
  console.log("Initial Supply:", ethers.formatUnits(initialSupply, tokenDecimals));

  // Get gas estimate
  const gasEstimate = await token.initialize.estimateGas(
    tokenName,
    tokenSymbol,
    tokenDecimals,
    initialSupply,
    wallet.address
  );

  console.log("Estimated Gas:", gasEstimate.toString());

  // Send transaction with 20% buffer
  const tx = await token.initialize(
    tokenName,
    tokenSymbol,
    tokenDecimals,
    initialSupply,
    wallet.address,
    { gasLimit: gasEstimate.mul(120).div(100) }
  );

  console.log("Transaction Hash:", tx.hash);
  console.log("Waiting for confirmation...");
  await tx.wait();

  console.log("✅ Mainnet initialization complete!");
}

main().catch(console.error);
```

### Step 6: Configure Production Features

```bash
# Run configuration script
node scripts/configure-mainnet.js
```

### Step 7: Verify on Mainnet Explorer

1. Go to [Arbiscan](https://arbiscan.io/)
2. Search for your contract address
3. Verify and publish source code

---

## Configuration Examples

### Example 1: Fixed Supply Token (Recommended for simplicity)

```javascript
// In your deployment script
token.initialize(
    "Fixed Token",
    "FIX",
    18,
    U256::from(1_000_000_000_000_000_000_000_000), // 1M fixed
    owner_address
);

// Enable supply cap to prevent any future minting
await token.setSupplyCapEnabled(true);

// Optionally renounce ownership to make it truly fixed
await token.renounceOwnership();
```

**Use Case:** Governance tokens, community tokens, fair launch tokens

### Example 2: Mintable Supply Token

```javascript
// In your deployment script
token.initialize(
    "Mintable Token",
    "MINT",
    18,
    U256::from(100_000_000_000_000_000_000_000), // 100K initial
    multisig_address // Use multi-sig for security
);

// Set supply cap
await token.setSupplyCap(ethers.parseUnits("1000000000", 18));
await token.setSupplyCapEnabled(true);

// Configure time-lock for ownership
await token.setOwnershipTransferDelay(48 * 60 * 60); // 48 hours

// DO NOT renounce - keep ownership for minting
```

**Use Case:** Rewards tokens, inflationary tokens, ecosystem tokens

### Example 3: Stablecoin-like Token

```javascript
// In your deployment script
token.initialize(
    "My Stablecoin",
    "MSC",
    6, // USDC uses 6 decimals
    U256::from(0), // Start with 0 supply
    treasury_multisig_address
);

// Enable strict supply controls
await token.setSupplyCap(ethers.parseUnits("10000000000", 6)); // 10B cap
await token.setSupplyCapEnabled(true);

// Enable blacklist for compliance
await token.setBlacklistEnabled(true);

// Multi-sig roles
await token.grantRole(MINTER_ROLE, treasury_multisig_address);
await token.grantRole(PAUSER_ROLE, emergency_multisig_address);

// Mint as needed based on reserves
// Implement strict minting controls with role limits
```

**Use Case:** Stablecoins, backed tokens, synthetic assets

### Example 4: Gaming/App Token

```javascript
// In your deployment script
token.initialize(
    "Game Token",
    "GAME",
    18,
    U256::from(1_000_000_000_000_000_000_000_000_000), // 1B tokens
    game_contract_address
);

// Game contract handles distribution
// Enable blacklist for cheater management
await token.setBlacklistEnabled(true);

// Set up guardian for emergency pause
await token.setGuardian(emergency_team_multisig);

// Consider implementing burn mechanisms for deflationary pressure
```

**Use Case:** In-game currencies, app tokens, utility tokens

---

## Production Checklist

### Pre-Deployment

- [ ] **Code Review**
  - [ ] All security requirements met
  - [ ] No TODO comments remaining
  - [ ] Code follows best practices
  - [ ] Documentation complete

- [ ] **Testing**
  - [ ] All unit tests passing
  - [ ] Integration tests passing
  - [ ] Testnet deployment successful
  - [ ] All functions tested
  - [ ] Edge cases covered

- [ ] **Security**
  - [ ] Professional audit completed
  - [ ] Bug bounty program launched
  - [ ] No critical vulnerabilities
  - [ ] Medium issues addressed

- [ ] **Configuration**
  - [ ] Token parameters finalized
  - [ ] Supply cap configured
  - [ ] Roles assigned
  - [ ] Time-lock configured
  - [ ] Guardian assigned

### Deployment

- [ ] **Environment**
  - [ ] Mainnet RPC verified
  - [ ] Gas prices checked
  - [ ] Hardware wallet ready
  - [ ] Backup wallet prepared

- [ ] **Execution**
  - [ ] Contract deployed
  - [ ] Token initialized
  - [ ] Production features configured
  - [ ] Transaction hashes saved

### Post-Deployment

- [ ] **Verification**
  - [ ] Contract verified on explorer
  - [ ] All functions callable
  - [ ] Events emitting correctly
  - [ ] State matches expectations

- [ ] **Monitoring**
  - [ ] Monitoring alerts configured
  - [ ] Dashboard set up
  - [ ] Emergency procedures tested

- [ ] **Communication**
  - [ ] Community announcement ready
  - [ ] Documentation published
  - [ ] Support channels ready

---

## Verification

### 1. Block Explorer Verification

```bash
# Export ABI
cargo stylus export-abi > abi.json

# On Arbiscan:
# 1. Go to your contract page
# 2. Click "Verify and Publish"
# 3. Select "Solidity (Single File)"
# 4. Upload src/lib.rs
# 5. Upload abi.json
# 6. Set compiler version (check Cargo.toml)
# 7. Submit
```

### 2. Functional Verification

```javascript
// scripts/verify-mainnet.js
const { ethers } = require('ethers');
require('dotenv').config();

const TOKEN_ADDRESS = process.env.TOKEN_ADDRESS;

const VERIFY_ABI = [
  "function name() view returns (string)",
  "function symbol() view returns (string)",
  "function decimals() view returns (uint8)",
  "function totalSupply() view returns (uint256)",
  "function balanceOf(address owner) view returns (uint256)",
  "function owner() view returns (address)",
  "function paused() view returns (bool)",
  "function supplyCap() view returns (uint256)",
  "function supplyCapEnabled() view returns (bool)",
  "function hasRole(uint32 role, address account) view returns (bool)"
];

async function verify() {
  const provider = new ethers.JsonRpcProvider(process.env.ARBITRUM_MAINNET_RPC);
  const token = new ethers.Contract(TOKEN_ADDRESS, VERIFY_ABI, provider);

  console.log("Verifying token at:", TOKEN_ADDRESS);

  const [name, symbol, decimals, totalSupply, owner, paused, supplyCap, supplyCapEnabled] =
    await Promise.all([
      token.name(),
      token.symbol(),
      token.decimals(),
      token.totalSupply(),
      token.owner(),
      token.paused(),
      token.supplyCap(),
      token.supplyCapEnabled()
    ]);

  console.log("\n=== Token Verification ===");
  console.log("Name:", name);
  console.log("Symbol:", symbol);
  console.log("Decimals:", decimals);
  console.log("Total Supply:", ethers.formatUnits(totalSupply, decimals));
  console.log("Owner:", owner);
  console.log("Paused:", paused);
  console.log("Supply Cap:", ethers.formatUnits(supplyCap, decimals));
  console.log("Supply Cap Enabled:", supplyCapEnabled);

  console.log("\n✅ Token verified successfully!");
}

verify().catch(console.error);
```

### 3. Event Verification

```javascript
// Check if initialization event was emitted
const initFilter = token.filters.Transfer(ethers.ZeroAddress, owner, null);
const initEvents = await token.queryFilter(initFilter);
if (initEvents.length > 0) {
  console.log("✅ Initialization event found");
}
```

---

## Troubleshooting

### Issue: "Already Initialized" Error

**Solution**: The token can only be initialized once. Deploy a new contract if needed.

```bash
# Deploy new contract
cargo stylus deploy --endpoint=$RPC_URL --contract-name=$CONTRACT_NAME
```

### Issue: "Not Owner" Error

**Solution**: The operation requires owner privileges. Check you're using the correct account.

```javascript
// Verify owner
const owner = await token.owner();
console.log("Expected owner:", expectedOwner);
console.log("Actual owner:", owner);
```

### Issue: Insufficient Gas

**Solution**: Increase gas limit in your transaction.

```javascript
const tx = await token.functionName(..., {
  gasLimit: gasEstimate.mul(150).div(100) // 50% buffer
});
```

### Issue: "Paused" Error

**Solution**: Token is currently paused. Contact owner to unpause.

```javascript
// Check pause status
const paused = await token.paused();
console.log("Token paused:", paused);

// If you're the owner, unpause
if (paused) {
  await token.unpause();
}
```

### Issue: "Insufficient Balance/Allowance"

**Solution**: Ensure sender has enough tokens and proper approvals are set.

```javascript
// Check balance
const balance = await token.balanceOf(sender);
console.log("Balance:", ethers.formatUnits(balance, decimals));

// Check allowance
const allowance = await token.allowance(sender, spender);
console.log("Allowance:", ethers.formatUnits(allowance, decimals));
```

### Issue: Role Not Granted

**Solution**: Grant the required role to the address.

```javascript
// Grant minter role
await token.grantRole(MINTER_ROLE, minterAddress);

// Check if role was granted
const hasRole = await token.hasRole(MINTER_ROLE, minterAddress);
console.log("Role granted:", hasRole);
```

### Issue: Supply Cap Exceeded

**Solution**: Increase the supply cap or reduce mint amount.

```javascript
// Current supply and cap
const [currentSupply, cap] = await Promise.all([
  token.totalSupply(),
  token.supplyCap()
]);

console.log("Current Supply:", ethers.formatUnits(currentSupply, decimals));
console.log("Supply Cap:", ethers.formatUnits(cap, decimals));

// If you need to increase the cap (can only increase, not decrease)
if (newAmount > cap) {
  await token.setSupplyCap(newCap);
}
```

---

## Emergency Procedures

### In Case of Emergency

1. **Pause the Token** (if you're the owner or have PAUSER_ROLE):
   ```javascript
   await token.pause();
   ```

2. **Guardian Pause** (if guardian is configured):
   ```javascript
   await token.guardianPause();
   ```

3. **Investigate the Issue**:
   - Check recent transactions
   - Verify contract state
   - Identify the problem

4. **Communicate**:
   - Notify users immediately
   - Explain the situation
   - Provide timeline for resolution

5. **Resolve and Unpause**:
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

### Emergency Contacts

- **Primary**: Contract Owner
- **Backup**: Emergency Admin
- **Guardian**: Designated guardian (if enabled)
- **Auditor**: Security audit firm
- **Legal**: Legal counsel

---

## Network-Specific Deployment

### Arbitrum Sepolia (Testnet)

```bash
RPC_URL=https://sepolia-rollup.arbitrum.io/rpc
CHAIN_ID=421614
EXPLORER=https://sepolia.arbiscan.io
FAUCET=https://sepolia.arbitrum.io/
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

---

## Gas Optimization Tips

1. **Batch Operations**: Use `batchTransfer()` and `batchApprove()`
2. **Use Events**: Events are cheaper than storing data
3. **Minimize Storage Writes**: Only write when necessary
4. **Zero Amount Checks**: Already optimized in implementation
5. **Allowance Patterns**: Use `increaseAllowance` instead of `approve(0)` then `approve(X)`

---

## Support Resources

- [Arbitrum Discord](https://discord.gg/arbitrum)
- [Stylus Documentation](https://docs.arbitrum.io/stylus)
- [Arbitrum Forum](https://forum.arbitrum.foundation/)
- [Rust Stylus SDK](https://docs.rs/stylus-sdk/)
- [Arbiscan](https://arbiscan.io/)

---

**Remember**: Deploying a token on mainnet is a serious responsibility. Always test thoroughly and consider professional security audits for production deployments.

**Final Checklist Before Mainnet**:

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
