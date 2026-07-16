#!/bin/bash

while [[ $(rsync --delete -ai wurstmineberg@wurstmineberg.de:/opt/wurstmineberg/world/$1/ $1/world/) ]]; do
    :
done
