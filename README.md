# sdsdata
Fetch data from Sigma bicycle computers using the Sigma Docking Station USB cradle, optionally zeroing counters:

* works on Linux (should work on every platform featuring Rust and Libusb)
* pipeable to other services (messages to stderr, data to stdout)
* does *not* support access/store data on Sigma Cloud

### hardware tested
* Sigma Docking Station TL2012 (USB id: `1d9d:1011`)
* Sigma BC 16.12
* expected to work with all Sigma BC xx.12 models (STS ones as well), Topline 2009/2012

### software dependencies
* written in [Rust](http://rust-lang.org)
* uses the [`libusb` crate](https://github.com/dcuddeback/libusb-rs)
* which in turn requires the native [`libusb-1.0`](http://libusb.org/) library

### build and install
* install libusb devel stuff (for example, in Ubuntu/Debian distros: `sudo apt install libusb-1.0-dev`)
* fetch this package: `git clone https://github.com/ciofeca/sdsdata`
* compile: `cd sdsdata; cargo build --release`
* move the executable in some convenient directory: `mv target/release/sdsdata ~/bin/`

Now you can use it using `sudo ~/bin/sdsdata`

### configuring udev to allow your user access libusb without root privileges
* either add your user to an "usb" group (using *groupadd* and *vigr* commands), or edit the `50-sdsdata.rules` to change the group "usb" to your user group id (gid) name (as shown by the *id* command in your shell)
* then copy the `50-sdsdata.rules` in the */etc/udev/rules.d*
* execute `sudo udevadm control --reload-rules`

From next login `~/bin/sdsdata` won't need *sudo* anymore.

### running it
* *sdsdata* will poll the cradle until some unit is inserted on one of its slots
* will identify it, fetch readable fields, and print them to stdout
* will optionally clear (zero) distance, time and speeds fields
* will optionally wait the user to remove the unit from the cradle

Available command-line arguments to change default "non-destructive read" behavior:

* `--clear` (resets unit's counters after printing them; equivalent to long-pressing the unit's top left button)
* `--dump` (outputs also an hex dump of identification and data packets)
* `--miles` (recalculates distances/speeds in miles instead of kilometers)
* `--no-ts` (does not print the Trip Section (ts) totals fields)
* `--no-zeros` (does not print fields with a zero value)
* `--raw` (only prints raw field values, comma-separated; distances will be in meters, times in seconds)
* `--wait-remove` (after printing data, don't exit until the unit is removed from the cradle)

### data read from the unit
* trip distance (meters, normally shown as x.xx kilometers)
* ride time (seconds, normally shown as h:mm:ss)
* average speed (x.xx km/hour)
* maximum speed (x.xx km/hour)
* cadence (x/minute, if sensor available)
* trip section distance (x.xx kilometers)
* trip section ride time (h:mm:ss)

Note: distance counters are always in meters, even if the unit is configured to show mph.

### usage examples
*`sdsdata`* (example after a quick ride)

        # cradle found on bus 002 device 061
        # unit found in the cradle
        # unit identified as a BC 16.12 type 0 version 22
        # unit serial number: 2221973
        distance: 6.21 km
        time: 0:17:39
        meanspeed: 21.13 km/hr
        maxspeed: 29.82 km/hr
        cadence: 71/min
        ts_dist: 98.58 km
        ts_time: 6:21:02

*`sdsdata --raw`* (same data as above; first field is meters, second field is seconds):

        # cradle found on bus 002 device 061
        # unit found in the cradle
        # unit identified as a BC 16.12 type 0 version 22
        # unit serial number: 2221973
        6215,1059,21.13,29.82,71,98583,22862

*`sdsdata --raw --clear --wait-remove >mydata.txt`* (still reading the same data, but now clearing counters and waiting; note that "#" messages are on stderr, while data contents are redirected to a text file):

        # cradle found on bus 002 device 061
        # resetting usb channel
        # unit found in the cradle
        # unit identified as a BC 16.12 type 0 version 22
        # unit serial number: 2221973
        # non-ts counters cleared
        # waiting for unit removal from the cradle
        # unit removed, exiting...

### typical "tweeting" or "database" setup (check [extra](extra/) files)
- cradle always connected to a 24/365 home server
- cradle normally empty
- service always running *(`while true; do sdsdata --clear --wait-remove | tweetdata.rb; done`)*
- go for a ride
- back home, insert unit in the cradle
- wait a few seconds for data read
- remove unit from the cradle

*sdsdata* ends, its output is fed to *tweetdata.rb* while service is restarted while the cradle is empty.

Note: *tweetdata.rb* requires [Oysttyer](https://github.com/oysttyer/oysttyer) command-line Twitter client.

[Sqlite3](https://www.sqlite.org/) database: same method, this time using *tripdatabase.rb* script (add *--quiet* to not to echo last inserted records).

Note: omit some *--clear* if you want to read the same data multiple times.

### license
Copyright (c) 2016 Alfonso Martone

Distributed under the [MIT License](LICENSE).
