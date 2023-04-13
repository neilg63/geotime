# GeoTime: Timezone data for any land location since 1900

This server application uses the open source time zone database and Geonames service to match any latitude, longitude and date since approximately 1900. However, the earliest time zone records vary considerably from place to place. Data is available for most of Europe, the Americas, Australia and many regions under European colonial control since at least 1900 and in some countries much earlier, e.g. 1835 for the Netherlands or 1847 for the UK, but there are many gaps in available records before 1930. When time zone data cannot be matched, standardised natural time zones to the nearest hour are used after 1900 (abbreviation LOC) and only local solar time before 1900 (abbreviation SOL).

## Build instructions:

You may use `cargo build (--release)` to build an executable for your operating system (all versions of Linux, Mac or Windows supported by Rust 1.61). This application requires MySQL or MariaDB. However, you will have to download and import the database (TimeZoneDB.sql.zip) from the [Timezone DB site](https://timezonedb.com/download).

```
mysql> create database timezonedb;
mysql> use timezonedb;
mysql> GRANT ALL PRIVILEGES ON timezonedb.* TO timezonedb@localhost IDENTIFIED BY 'my_cryptic_password';
```

Exit the mysql prompt and import the SQL file as follows:

```
mysql -u timezonedb -pmy_cryptic_password timezonedb < time_zone.sql
```

## Environment Variables

The application will pick up a .env file in the launch directory, which is assumed to be the project root where the executable is at target/release/geotimezone.

- port: Web server port number, default 8809
- db_name: database name, default timezonedb
- db_user: database user name, default timezonedb
- db_pass= database password, default password (not use this)
- db_port: database port, default 3306.
- db_host: database host "127.0.0.1"
- geonames_user_name: registered GeoNames name, default demo (only temporary). [Geonames user name](https://www.geonames.org/login). NB. This is free..
- max_nearby_radius: Kilometers from nearest continental area with an official timezone, default 240. Only used for locations at sea.

## Command line parameters

These will override the above.

- -d: MySQL database name, default: timezonedb (table name time_zone)
- -u: MySQL user name, default: timezonedb
- -p: MySQL password, default: password. Must be configured
- -h: MySQL Host, default 127.0.0.1
- -P: MySQL Port number default 3306
- -w: Web port for the server, default: 8089
- -g: [Geonames user name](https://www.geonames.org/login). NB. This is free.

## Endpoints

### GET timezone

This shows the timezone, offsets and local time in various formats for the referenced location and date-time

Query string parameters

- loc: Comma-separated decimal latitude and longitude
- dt: UTC date or date-time as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- dtl: Local date or date-time expressed as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- jd: Decimal julian day as an alternative to datetime
- un: Unix timestamp. Dates before midnight 1 Jan 1970 UTC are negative integers.

The current time will be used if none is specified.

#### Response:

- zoneName: _Continent|Sea|Ocean/City|Segment_
- abbreviation: 3 or 4 letter uppercase time zone abbreviation. However, their definition may change over time and a time zone region (see below) may switch time zones, change daylight saving rules or redefine the offset from UTC. When unmatched, "SOL" means solar time to the nearest second and "LOC" stands for standardised longitude-based local time to the nearest hour.
- countryCode: 2-letter country code (NB: the code assigned to some regions may be contested, e.g. Crimea, or reflect current geopolitcial boundaries rather than those valid at the time)
- dst: boolean true/false for daylight saving time or summer time
- gmtOffset: seconds difference from UTC. These are usually rounded to the nearest hour (3600 seconds) and less commonly to the nearest half hour (India, South Australia) or quater hour (Nepal)
- localDt: The calculated local datetime string
- utc: The calculated UTC datetime string
- refJd: The calculated Julian day of the UTC date-time
- refUnix: The calculated unix time stamp
- solarUtcOffset: The offset from UTC as it should be by longitude alone, ensuring noon or 12am is where the sun reaches its highest point.
- period.start: Start of this time offset as a unix timestamp (if known)
- period.startUtc: Time offset start as as UTC date-time string (if known)
- period.nextGmtOffset: The next gmt offset in seconds at the end of the current period
- period.end: End of this time offset as a unix timestamp (if known)
- period.endUtc: Time offset end as as UTC date-time string (if known). For regions that do not apply summer time (daylight saving), the time offset is assumed to remain the same until further notice.
- weekDay.abbr: Three-letter English abbreviation of the local week day
- weekDay.iso: ISO day of the week, where 1 = Monday and 7 = Sunday
- weekDay.sun: Alternative weekday number where Sunday = 1 and Saturday = 7 (common in the Americas and India)

### GET geotime

This shows the timezone, offsets and local time in various formats for the referenced location and date-time with related place names.

Query string parameters

- loc: Comma-separated decimal latitude and longitude
- dt: UTC date or date-time as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- dtl: Local date or date-time expressed as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- jd: Decimal julian day as an alternative to datetime
- un: Unix timestamp. Dates before midnight 1 Jan 1970 UTC are negative integers.
- zn: Canonical zone name if known e.g. Asia/Kolkata or Europe/Amsterdam (this will avoid an extra GeoNames lookup and may be marginally faster)

If no time is specified, the current time will be used.

### Response

- placenames: Set of related place names from country to locality level or ocean if out at sea.
- time: As above with GET /timezone
