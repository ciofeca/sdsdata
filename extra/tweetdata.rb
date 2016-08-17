#!/usr/bin/env ruby

# typical usage:  $HOME/bin/sds-data --clear --wait-remove | $HOME/bin/tweetdata.rb

r = STDIN.readlines
exit 1  if r==nil || r.empty?

s = r.collect { |i| i.chomp.split ": " }
# [["distance", "3.06 km"], ["time", "0:12:32"], ["meanspeed", "15.27 km/hr"], ["maxspeed", "23.19 km/hr"], ["cadence", "63/min"], ["ts_dist", "97.08 km"], ["ts_time", "6:11:53"]]

t = "Today's #cycling stats: #{s[0].last} in #{s[1].last} (mean: #{s[2].last}, max: #{s[3].last})"
t += "; cadence: #{s[4].last}"  if s[4].first=='cadence'
t += ". Grand total: #{s[-2].last} in #{s[-1].last}"

puts "!--tweeting[#{t.size}]: #{t}"
sleep 3

# oysttyer is a command-line twitter client ( https://github.com/oysttyer/oysttyer )
# we use it in non-interactive mode to tweet the text string just built:
system "oysttyer -silent -status=#{t.inspect} -ssl"
