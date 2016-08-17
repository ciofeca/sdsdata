#!/usr/bin/env ruby

# typical usage:  $HOME/bin/sds-data --raw --clear --wait-remove | $HOME/bin/tweetdata.rb
# tested with sqlite 3.11

r = STDIN.gets
exit 1  if r==nil || r.empty?

opt   = '-column'
extra = 'select "total records:", count(*) from data;
         select "last records:";
         select * from data order by ts desc limit 4;'

if ARGV.first == '--quiet'
  opt = ''
  extra = ''
end
  
system "sqlite3 #{opt} tripdatabase.sqlite3 '
  create table if not exists  data (
    ts         timestamp primary key,
    meters     integer,
    seconds    integer,
    meanspeed  float,
    maxspeed   float,
    cadence    integer,
    ts_dist    integer,
    ts_time    integer,
    constraint ride unique (meters, seconds, meanspeed));

  insert into data (ts, meters, seconds, meanspeed, maxspeed, cadence, ts_dist, ts_time)
    values     (datetime(current_timestamp, \"localtime\"), #{r.chomp});

  #{extra}'"

