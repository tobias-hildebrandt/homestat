#!/bin/sh

if [ "$(id -u)" -ne 0 ]; then
    echo "must be root"
    exit
fi

username=$(who am i | awk '{print $1}')

echo "user: $username"

user_id=$(id -u "$username")
group_id=$(id -g "$username")

mount_location=$(realpath mount)

DEVICE=$(lsblk -o KNAME,MODEL | grep RP2 | awk '{print $1}')
PART=/dev/"$DEVICE"1

echo "rpi partition: $PART"

sudo mount $PART $mount_location -o uid="$user_id",gid="$group_id"

if [ ! -f "$mount_location/INDEX.HTM" ]; then
    echo "failed, cannot see INDEX.HTM in mount location"
    exit
else
    echo "success, mounted at $mount_location"
fi
