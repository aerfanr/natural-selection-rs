#!/bin/bash

i=0

while [ $i -lt $1 ] 
do
    timeout 5 cargo run | tail -n1
    ((i++))
done