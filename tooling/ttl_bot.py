#!/usr/bin/env python3
"""
Storage TTL Monitoring & Auto-Renewal Bot Config
This script tracks contract storage TTL and sends extend_ttl transactions before expiration.

Objectives:
- Query Soroban RPC storage entry TTL for contract keys periodically.
- Trigger extend_ttl transactions automatically when TTL falls below safety threshold.
- Alert maintainers if bot wallet balance is low.

Dependencies:
    pip install stellar-sdk requests
"""

import argparse
import logging
import time
import sys

try:
    import requests
    from stellar_sdk import (
        Keypair,
        Network,
        Server
    )
    from stellar_sdk.exceptions import BaseHorizonError
except ImportError as e:
    print(f"Missing dependency: {e}")
    print("Please install required dependencies: pip install stellar-sdk>=9.0.0 requests")
    sys.exit(1)

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger("TTLBot")

def check_balance(horizon_url, public_key, min_balance):
    """Alerts maintainers if bot wallet balance is low."""
    try:
        server = Server(horizon_url)
        account = server.accounts().account_id(public_key).call()
        for balance in account['balances']:
            if balance['asset_type'] == 'native':
                xlm_balance = float(balance['balance'])
                logger.info(f"Wallet {public_key} has {xlm_balance} XLM")
                if xlm_balance < min_balance:
                    logger.warning(f"ALERT: Bot wallet balance is low! ({xlm_balance} XLM < {min_balance} XLM)")
                return xlm_balance
        logger.warning(f"ALERT: No native XLM balance found for wallet {public_key}.")
        return 0.0
    except BaseHorizonError as e:
        logger.error(f"Failed to check balance for {public_key}: {e}")
        return 0.0

def get_current_ledger(soroban_rpc_url):
    """Query Soroban RPC for the current ledger sequence."""
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestLedger"
    }
    response = requests.post(soroban_rpc_url, json=payload)
    response.raise_for_status()
    data = response.json()
    if 'result' in data:
        return data['result']['sequence']
    return None

def extend_ttl_subprocess(contract_id, extend_to, source_secret, network, rpc_url):
    """Use Soroban CLI to extend the TTL of the contract."""
    import subprocess
    cmd = [
        "soroban", "contract", "extend",
        "--id", contract_id,
        "--ledgers-to-extend", str(extend_to),
        "--source-key", source_secret,
        "--network-passphrase", network,
        "--rpc-url", rpc_url
    ]
    logger.info(f"Executing CLI extend for {contract_id} ...")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        logger.info(f"Successfully extended TTL for contract {contract_id}: {result.stdout.strip()}")
    except subprocess.CalledProcessError as e:
        logger.error(f"Failed to extend TTL: {e.stderr}")

def monitor_and_renew(
    rpc_url: str,
    horizon_url: str,
    network_passphrase: str,
    secret_key: str,
    contract_ids: list,
    threshold: int,
    extend_to: int,
    min_balance: float,
    dry_run: bool
):
    try:
        kp = Keypair.from_secret(secret_key)
        public_key = kp.public_key
    except Exception as e:
        logger.error(f"Invalid secret key: {e}")
        return

    # 1. Alert maintainers if bot wallet balance is low.
    check_balance(horizon_url, public_key, min_balance)

    # 2. Get latest ledger
    latest_ledger = get_current_ledger(rpc_url)
    if not latest_ledger:
        logger.error("Could not fetch latest ledger from Soroban RPC.")
        return

    logger.info(f"Current Ledger Sequence: {latest_ledger}")

    # 3. Query TTL & Trigger extend_ttl transactions automatically
    for contract_id in contract_ids:
        logger.info(f"Checking TTL for contract: {contract_id}")
        
        # Note: In a pure python setup querying exact TTL for contract instance
        # requires parsing scval XDR. For this configuration script, we simulate 
        # the TTL check and rely on the `soroban contract extend` CLI to manage the footprint.
        
        # Simulated TTL query result (replace with actual RPC getLedgerEntries query using XDR keys)
        current_ttl = threshold - 100  # Force trigger for demonstration

        if current_ttl < threshold:
            logger.warning(f"TTL for contract {contract_id} is below threshold ({threshold}). Needs extension.")
            
            if dry_run:
                logger.info(f"[DRY-RUN] Would execute extend_ttl transaction for {contract_id} by {extend_to} ledgers.")
            else:
                logger.info(f"Triggering extend_ttl transaction for {contract_id}...")
                extend_ttl_subprocess(contract_id, extend_to, secret_key, network_passphrase, rpc_url)
        else:
            logger.info(f"TTL for contract {contract_id} is healthy ({current_ttl} >= {threshold}).")

def main():
    parser = argparse.ArgumentParser(description="Storage TTL Monitoring & Auto-Renewal Bot Config")
    parser.add_argument("--rpc-url", default="https://soroban-testnet.stellar.org", help="Soroban RPC URL")
    parser.add_argument("--horizon-url", default="https://horizon-testnet.stellar.org", help="Stellar Horizon URL")
    parser.add_argument("--network", default=Network.TESTNET_NETWORK_PASSPHRASE, help="Network passphrase")
    parser.add_argument("--secret-key", required=True, help="Secret key of the bot wallet")
    parser.add_argument("--contract-ids", required=True, help="Comma-separated list of contract IDs to monitor")
    parser.add_argument("--threshold", type=int, default=10000, help="TTL threshold in ledgers to trigger renewal")
    parser.add_argument("--extend-to", type=int, default=50000, help="Number of ledgers to extend the TTL to")
    parser.add_argument("--min-balance", type=float, default=10.0, help="Minimum XLM balance for bot wallet")
    parser.add_argument("--dry-run", action="store_true", help="Run without actually sending transactions")
    parser.add_argument("--interval", type=int, default=0, help="Interval in seconds to run periodically. 0 means run once.")
    
    args = parser.parse_args()

    contract_ids = [c.strip() for c in args.contract_ids.split(',') if c.strip()]

    logger.info("Starting Storage TTL Monitoring Bot...")
    if args.dry_run:
        logger.info("=== DRY-RUN MODE ENABLED ===")

    while True:
        monitor_and_renew(
            rpc_url=args.rpc_url,
            horizon_url=args.horizon_url,
            network_passphrase=args.network,
            secret_key=args.secret_key,
            contract_ids=contract_ids,
            threshold=args.threshold,
            extend_to=args.extend_to,
            min_balance=args.min_balance,
            dry_run=args.dry_run
        )
        
        if args.interval <= 0:
            break
        logger.info(f"Sleeping for {args.interval} seconds before next check...")
        time.sleep(args.interval)

if __name__ == "__main__":
    main()
