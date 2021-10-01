#!/bin/bash

# Album with 1. Filenames in track num 2. Track numbers in ID3 tags
# 3. Files created in track number order
mkdir -p album_with_ordered_filenames
for track_num in `seq 1 20`; do
	ffmpeg -f lavfi -i anullsrc=r=44100:cl=mono -t 1 -q:a 9 \
		-acodec libmp3lame album_with_ordered_filenames/$track_num.mp3
	mid3v2 -a "Artist" -A "Album with ordered track numbers" \
		-t "Song $track_num" -T $track_num album_with_ordered_filenames/$track_num.mp3
done

# Album with 1. Filenames unordered 2. Track numbers in ID3 tags
# 3. Files created in track number order
mkdir -p album_with_unordered_filenames
for track_num in `seq 1 20`; do
    filename=$(mktemp -u --suffix=.mp3 -p album_with_unordered_filenames)
	ffmpeg -f lavfi -i anullsrc=r=44100:cl=mono -t 1 -q:a 9 \
		-acodec libmp3lame $filename
	mid3v2 -a "Artist" -A "Album with unordered track numbers" \
		-t "Song $track_num" -T $track_num $filename
done

# Album with 1. Random filenames. 2. Track numbers in ID3 tags
# 3. Random creation order
mkdir -p album_with_random_ctime
for track_num in `seq 1 20 | shuf`; do
    filename=$(mktemp -u --suffix=.mp3 -p album_with_random_ctime)
	ffmpeg -f lavfi -i anullsrc=r=44100:cl=mono -t 1 -q:a 9 \
		-acodec libmp3lame $filename
	mid3v2 -a "Artist" -A "Album with random ctime"\
		-t "Song $track_num" -T $track_num $filename
		echo $filename
	done

# Album with good filenames but no ID3 tags.
# Album with 1. Filenames in track num 2. Track numbers in ID3 tags
# 3. Files created in track number order
mkdir -p album_with_no_tags
for track_num in `seq 1 20`; do
	ffmpeg -f lavfi -i anullsrc=r=44100:cl=mono -t 1 -q:a 9 \
		-acodec libmp3lame \
		"album_with_no_tags/$track_num. Song $track_num.mp3"
done

