#!/bin/sh

EXPORT_DPI="72"
EXPORT_SIZE="64"

# delete all duplicates
# find assets/icons/ -name '*(1).svg' -delete

for dir in assets/icons/*; do
  for file in $dir/*.svg; do

    if [ "$dir" == "assets/icons/home" ]; then
      EXPORT_SIZE="128"
    fi

    # assets -> ui/images
    export_file="${file/assets/ui\/images}"
    # remove _24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24
    export_file="${export_file/_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24/}"
    # svg -> png
    export_file="${export_file/svg/png}"


    if ! [ -f "$file" ]; then
      echo "$file does not exist, probably a script problem"
      continue
    fi

    if [ -f "$export_file" ] && ! [ "$1" == "--force" ]; then
      # echo "$export_file already exists, no action required"
      continue
    fi

    echo $file
    echo "-> $export_file"


    inkscape --export-dpi=$EXPORT_DPI --export-width=$EXPORT_SIZE --export-filename="${export_file}" "$file";
  done
done


