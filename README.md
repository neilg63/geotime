# GeoTimeZone: Timezone data for any land location since 1900

A preview of some of the features is available at [GeoTimeZone service](https://geotimezone.multifaceted.info/).

This server application uses the open source time zone database and Geonames service to match any latitude, longitude and date since approximately 1900. However, the earliest time zone records vary considerably from place to place. Data is available for most of Europe, the Americas, Australia and many regions under European colonial control since at least 1900 and in some countries much earlier, e.g. 1835 for the Netherlands or 1847 for the UK, but there are many gaps in available records before 1930. When time zone data cannot be matched, standardised natural time zones to the nearest hour are used after 1900 (abbreviation LOC) and only local solar time before 1900 (abbreviation SOL).

## What problem does GeoTimeZone solve?

While many existing tools, fully integrated with your device's operating system, make it easy to find the current universal (UTC) time for your location or for any identifiable time zone, it is not as easy to find time zone data if you only know the latitude and longitude without querying first a location search service and then a separate time zone database. Additionally, time zones and daylight saving times have regularly changed for historic local times in many countries and regions. Most such shifts may only be 1 or 2 hours (although Western Samoa famously moved the clocks forward 23 hours in 2012), but this can make a significant difference when we need an exact chronology of events or an accurate UTC date-time, unix timestamp or Julian day for applications such as astrology.

## Summer or Daylight Saving Time

Reported local times near daylight-saving-time boundaries are problematic when specifying local time via the _dtl_ parameter in the /geotime and /timezone endpoints. If the clocks go forward at 01:00:00 (1am), there are no official local times between 01:00:00 and 01:59:59. The clock jumps from 00:59:59 to 02:00:00. The logic applied here assumes 01:30:00 is the same UTC time as 02:30:30. However, when the clocks go back at 02:00:00, the clock jumps from 01:59:59 to 01:00:00. This means 01:30:00 occurs twice once with _summer time_ and once without. In this case the logic applied assumes _summer time_ continues until the end of the overlapping period, unless the _dst_ parameter is set to 0 to cover the skipped UTC hour. Using the previous example, _dtl=2022-10-30T01:30:00&dst=0_ is the hour after _dtl=2022-10-30T00:30:00&dst=1_ (the default without the _dst_ parameter).

## Build instructions:

You may use `cargo build (--release)` to build an executable for your operating system (all versions of Linux, Mac or Windows supported by Rust 1.63+). This application requires MySQL or MariaDB. However, you will have to download and import the database (TimeZoneDB.sql.zip) from the [Timezone DB site](https://timezonedb.com/download).

```
mysql> create database timezonedb;
mysql> use timezonedb;
mysql> GRANT ALL PRIVILEGES ON timezonedb.* TO timezonedb@localhost IDENTIFIED BY 'my_cryptic_password';
```

Exit the mysql prompt and import the SQL file as follows:

```
mysql -u timezonedb -pmy_cryptic_password timezonedb < time_zone.sql
```

I bundled a timezone database with the _time_zone_ and _country_ tables plus a _cities_ lookup table from GeoNames for the localities endpoint

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

These will override the above and serve mainly for testing purposes.

- -d: MySQL database name, default: timezonedb (table name time_zone)
- -u: MySQL user name, default: timezonedb
- -p: MySQL password, default: password. Must be configured
- -h: MySQL Host, default 127.0.0.1
- -P: MySQL Port number default 3306
- -w: Web port for the server, default: 8089
- -g: [Geonames user name](https://www.geonames.org/login). NB. This is free.

## Endpoints

### GET /timezone

This shows the timezone, offsets and local time in various formats for the referenced location and date-time

Query string parameters

- loc: Comma-separated decimal latitude and longitude
- dt: UTC date or date-time as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- dtl: Local date or date-time expressed as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- jd: Decimal julian day as an alternative to datetime
- un: Unix timestamp. Dates before midnight 1 Jan 1970 UTC are negative integers.
- zn: Canonical zone name if known e.g. Asia/Kolkata or Europe/Amsterdam. This serves as an alternative to _lat,lng_ coordinates and avoids an extra GeoNames lookup and may hence be marginally faster.
- zn: Zone name, e.g. Asia/Kolkata:
- place: Place name search string, only used in combination with the _cc_ for country code, as an alternative to coordinates or zone names. This works best for major towns and cities. To avoid conflicts in countries with multiple time zones, you may specify a region with the _reg_ parameter.
- cc: Country code, required with the _place_ parameter for this endpoint
- reg: Region (state, province) optionally used with _place_ parameter

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

### GET /geotime

This shows a set of related _placenames_ (from country to locality) and timezone (_time_) data as described above.

Query string parameters

- loc: Comma-separated decimal latitude and longitude
- dt: UTC date or date-time as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- dtl: Local date or date-time expressed as yyyy-mm-dd (2000-01-01) or yyyy-mm-ddTHH:MM:SS (2000-01-01T12:00:00) with optional seconds
- jd: Decimal julian day as an alternative to datetime
- un: Unix timestamp. Dates before midnight 1 Jan 1970 UTC are negative integers.

If no time is specified, the current time will be used.

#### Response

- placenames: Set of related place names from country to locality level or ocean if out at sea.
- time: As above with GET /timezone

### GET /search

This provides a complementary placename search endpoint, leveraging GeoNames' [search service](http://www.geonames.org/export/geonames-search.html) with a slightly simpplifed output and set of options

Query string parameters

- place: Search string, which may include country or region names for disambiguation
- cc: Optional two-letter country code to narrow searches to a given country
- fuzzy: on a scale from 0 to 100, 100 is the maximum tolerance of spelling and name association and 0 for exact matches only. The default is 100
- included: 0 (default) include localities and regions and countries only, 1: include all topographic features such as buildings, airports, lakes and seas
- max: number of results between 1 and 255, default: 50

#### Response

- results: Array of related place names from country to locality level or ocean if out at sea.
- count: Number of matches

### GET /lookup

This provides simplified list of deduplicated place names, matching the letters after the place parameter in the query string. It is ideal for auto-complete lookups where you want to match a place name as entered with exact geographic coordinates.

Query string parameters

- place: Search string, which may include country or region names for disambiguation
- cc: Optional two-letter country code to narrow searches to a given country
- fuzzy: on a scale from 0 to 100, 100 is the maximum tolerance of spelling and name association and 0 for exact matches only. The default is 100
- max: number of results between 1 and 255, default: 20

### GET /localities

This provides a list of deduplicated place names, matching the letters after the place parameter in the query string. It is ideal for auto-complete lookups where you want to match a place name as entered with exact geographic coordinates.

Query string parameters

- place: Search string, which may include country or region names for disambiguation
- cc: Optional two-letter country code to narrow searches to a given country
- fuzzy: on a scale from 0 to 100, 100 is the maximum tolerance of spelling and name association and 0 for exact matches only. The default is 100
- max: number of results between 1 and 255, default: 20

#### Response

- Array of objects with text (adminName, countryCode, name, fcode, population, lat(itude) and l(o)ng(itude) and zoneName.
