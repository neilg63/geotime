# GeoTime: Timezone data for any land location since 1920

This server application uses the open source time zone database and Geonames service to match any latitude, longitude and date since approximately 1920 with accurate time zone data for the time and place in history.

## Build instructions:
You may use `cargo build (--release)` to build an executable for your operating system (all versions of Linux, Mac or Windows supported by Rust 1.61). This application requires MySQL or MariaDB. However, you will have to download and import the timezone database. 

```
mysql> create database timezonedb;
mysql> use timezonedb;
mysql> GRANT ALL PRIVILEGES ON timezonedb.* TO timezonedb@localhost IDENTIFIED BY 'my_cryptic_password'; 
```

Exit the mysql prompt and import the SQL file as follows:
```
mysql -u timezonedb -pmy_cryptic_password timezonedb < time_zone.sql
```


## Commad line parameters

* -d: MySQL database name, default: timezonedb (table name time_zone)
* -u: MySQL user name, default: timezonedb
* -p: MySQL password, default: password. Must be configured
* -h: MySQL Host, default 127.0.0.1
* -P: MySQL Port number default 3306
* -w: Web port for the server
* -g: Geonames user name. NB. This is free.

## Endpoints

### GET timezone

This shows the timezone, offsets and local time in various formats for the referenced location and date-time

Query string parameters

* loc: Comma-separated decimal latitude and longitude
* dt: UTC date or date-time as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
* jd: Decimal julian day as an alternative to datetime

If no time can be matched, the current time will be used;

### GET geotime

This shows the timezone, offsets and local time in various formats for the referenced location and date-time with related place names.

Query string parameters

* loc: Comma-separated decimal latitude and longitude
* dt: UTC date or date-time as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
* jd: Decimal julian day as an alternative to datetime
* zn: Canonical zone name if known e.g. Asia/Kolkata or Europe/Amsterdam (this will skip a GeoNames lookup and maybe marginally faster)

If no time can be matched, the current time will be used;