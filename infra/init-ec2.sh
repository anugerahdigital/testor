#!/bin/bash -xe

#
# provision ec2 instance with necessary deps
# instance: g4dn.xlarge
# ami: amazon linux 2023
#

# install system deps
yum update

yum install -y pip htop vulkan

yum groupinstall -y "Development Tools"

yum install -y kernel-devel-$(uname -r) kernel-headers-$(uname -r)

# https://github.com/amazonlinux/amazon-linux-2023/issues/538
dnf install -y kernel-modules-extra

yum remove -y python3-requests

# install nvidia driver
DRIVER_VERSION=535.54.03
curl -fSsl -O "https://us.download.nvidia.com/tesla/${DRIVER_VERSION}/NVIDIA-Linux-x86_64-${DRIVER_VERSION}.run"

sh NVIDIA-Linux-x86_64-$DRIVER_VERSION.run -s
rm -rf NVIDIA-Linux-x86_64-$DRIVER_VERSION.run 

# install cuda
CUDA_VERSION=12.2.0
curl -fSsl -O "https://developer.download.nvidia.com/compute/cuda/${CUDA_VERSION}/local_installers/cuda_${CUDA_VERSION}_${DRIVER_VERSION}_linux.run"

sh cuda_${CUDA_VERSION}_${DRIVER_VERSION}_linux.run --toolkit --silent
rm -rf cuda_${CUDA_VERSION}_${DRIVER_VERSION}_linux.run 

# add this to .bashrc
echo 'export PATH=/usr/local/cuda/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc


# install app specific deps

# install rust
curl https://sh.rustup.rs -sSf | sh -s -- -y
echo 'export PATH=$HOME/.cargo/bin:$PATH' >> ~/.bashrc

# resource bashrc to pickup new env vars
source ~/.bashrc

# to build the binary
yum install openssl-devel -y

# install solana for helpful commands
sh -c "$(curl -sSfL https://release.solana.com/v1.18.4/install)"

# clone the repo
git clone https://github.com/tonyke-bot/ore-miner.git
pushd ore-miner
cargo build --release
./build-cuda-worker.sh

echo 'init-ec2.sh complete'