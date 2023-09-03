# Diplaying bus times on 3 led matrices 64x64

```
Note: The code needs some cleanup, I got it working as fast as possible
```
## Demo

A demo is available here: [https://www.youtube.com/watch?v=ucZTBreqVV4](https://www.youtube.com/watch?v=ucZTBreqVV4)

## Goal 
The goal of this project was to learn the Rust programming language and to make something useful at the same time: dispalying the bus and tram times.

## Content / Technical part

This all project runs on a raspberrypi 3B; consuming 80% of 1 thread.
The total power consumptions is ~6W (5V * 1.2A).

### Controling the leds
I'm using [this C library](https://github.com/hzeller/rpi-rgb-led-matrix)  to control led matrices that I bought from Aliexpress (similar to [these ones](https://t.ly/BmLKT))
I used bindgen to generate Rust bindings to the C library.
I provided the headers + allowlist of the function to expose.
From the led lib repo; I built the shared library, and I export LD_LIBRARY_PATH to add that lib path when running the bin.

### Fetching the tram times

I'm using a swiss open source data website that answers get requests with Json, and then I'm parsing the json.
I'm doing this using Tokkio async functions and serde for the json serialization.
See [https://transport.opendata.ch/docs.html](https://transport.opendata.ch/docs.html) for API usage.

Note: there is a rate limit of 1000 requests per day.
For my use case that's more than enough.
I'm requesting for the next 10 buses/trams and I store that information.
I'm only performing request every 10 minutes, to get a more accurate information about delays.
With 5 routes, it means 720 requests per day (5 routes * 6 requests/hour/route  * 24h = 720)

### Autostart

I simply created a cron task to start that program when the raspberry pi boot.

### Hardware

There are 3 led panels of 64x64 led. They need 5V input.
There is a tiny red led 3V that just shows if some current is flowing through. 
I'm using [this adapter](https://github.com/hzeller/rpi-rgb-led-matrix/tree/master/adapter/active-3) to link the rasperry pi with the panels.
There was a tiny bit of soldering to select the pins that need to be used for [these panel](https://t.ly/BmLKT).

An important thing to know is that the rasperry pi and the panel GND must be linked together; otherwise there will be a strong flickering.
I was using a modular Power dispenser for the leds at the beginning and a power bank for the raspberry pi and that was causing a lot of issues.
If the power supply is not providing a stable voltage, some flickering can occur. That's an issue when using cheap modular power supplied.
I bought a  50W (5V 10A) good quality power supply later on and it was way better.

The wood frame was just glued around the 3 panels and reinforced with flat metal corners.
The raspberry pi is glued with hot glue behind the panel on its ends.
I don't need to access it physically as I can just ssh to it.

### Why doing it in Rust?

I'm mainly a C++ developper.
When it comes to projects that must come out quicly and be robust over long durations, some memory issues can easily happen by inadvertence.
Using another language with a GC would just make things slower and would not solve all the issues.
Rust allows to catch most of the problems at compile time.
The only remaining issues are mostly due to logic/math problems.

Also at design phase I was thinking the architecture a certain way as I would do in C++ (sharing mutable objects on different threads), and the compiler was not happy about it.
As I couldn't compile I found another way, that was indeed safer. (The main thread is now just using futures to call the timesheet API and checking for value availability at each iteration)

The only risky part in the code are the call to the C API in these unsafe scopes.

The panel has been running for 1 week now and everything is perfectly fine.

## Future improvement

* Dockerize the app
* Allow cross-compilation (compiling all rust libs on raspi takes ages)
* Expose a simple http server to control the panel with a phone from the local wifi
  * Update the panel
  * Make the destinations displayed configurable
* Cleanup the code and dependencies
  
