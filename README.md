# Logbook

A CLI based logger for flight simulators.

## Features

- Tracks flights from startup to shutdown.
- Handles touch and go's.
- Exports each flight to a CSV file.
- Does nothing else.

## Supported Simulators

- MSFS (SimConnect)
- X-Plane 12 (WIP)

## MSFS

### Requirements

- SimConnect.dll
- [navdatareader][1]

### Building

```
> cp C:\Path\To\SDK\SimConnect.dll .
> cargo build --release
> cargo install
```

### Running

To run the logbook you need to generate a `navdata.sql` file first with [navdatareader][1]:

```
> navdatareader.exe -f MSFS
```

After the navdata has been generated we can copy it over and run the logbook.

```
> cp C:\NavDataReader\navdata.sql .
> logbook.exe
```

## License

[GPLv3](LICENSE)

[1]: https://github.com/albar965/navdatareader
