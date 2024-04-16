# infra


### commands

```bash
# ssh
ssh -i miner.pem ec2-user@$HOST

# scp local to remote 
scp -i miner.pem $LOCAL_PATH ec2-user@$HOST:/home/ec2-user/$REMOTE_PATH

# run it
CUDA_VISIBLE_DEVICES=0 cargo run --release -- \
    --rpc https://jmine-main465-387a.mainnet.rpcpool.com/9e9efec3-d49a-4b4f-af70-dd81b73ffd0f \
    --priority-fee 2000000 \
    bundle-mine-gpu \
    --max-adaptive-tip 3000000
```