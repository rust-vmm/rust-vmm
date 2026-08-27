#! /bin/sh

filename=./vfio-ioctls/Cargo.toml
replace_crate() {
    line=$(grep -n "$1 = { version =" $filename | tail -n1 | cut -f1 -d:)
    if [ -z $line ]; then
            echo "$1 not found in the Cargo.toml file"
            exit 1
    fi
    sed -i "$line i $1 = { path = \"..\/..\/mshv\/$1\" $2" $filename
    line=$((line+1))
    sed -i "${line}d" $filename
}

replace_crate mshv-ioctls ', optional  = true }'
replace_crate mshv-bindings ', features = ['