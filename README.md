vcr
===
A tool for converting VHS tapes over a capture device using ffmpeg.

This is a very light layer over ffmpeg that is tightly constrained to my own
workflow. It's unlikely to be useful in a general context, but others might
benefit from seeing my process. Basically:

1. Start the VCR and this program at the same time. e.g.,
   `vcr -t 3:00:00 me.mp4` to record for 3 hours and then stop.
2. Do an initial capture into an `mkv` format. This permits watching the
   capture while ffmpeg is encoding it.
3. Blend the the final capture with a still "blue" image, and run ffmpeg's
   `blackdetect` filter over it. The result of this should tells us where
   the actual video ends.
4. Trim the trailing blue frames and, in the process, re-encode the video as
   mp4 for maximum portability.

Various ffmpeg settings are hard-coded, although some things---like video or
audio device location---can be controlled on the command line.

Each step of the above process is intended to be idempotent and the process can
be resumed via the `--resume` flag. Otherwise, a new job will always be
created. Generally speaking, `vcr` will preserve every intermediate artifact.
The motivation for this is that capturing video is a time intensive process, so
bugs later in the pipeline shouldn't cause outputs earlier in the pipeline to
become unusable.


Maintenance
===========
This project is intended for my own personal use, and I likely won't accept
feature requests or bug reports. I post it here for the purposes of sharing
only.
