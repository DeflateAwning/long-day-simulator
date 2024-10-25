from datetime import datetime, time, timedelta, date, timezone
import dataclasses
from typing import Literal, Union
from pathlib import Path

from ics import Calendar, Event

@dataclasses.dataclass
class SimConfig:
    bedtime_on_start_day: time
    start_day: date # "today"
    sleep_duration: timedelta
    day_length: timedelta # like "25 hours"
    stop_date_time: datetime # end the simulation here

    @property
    def awake_duration(self) -> timedelta:
        return self.day_length - self.sleep_duration

@dataclasses.dataclass
class Period:
    start: datetime
    end: datetime

    @property
    def duration(self) -> timedelta:
        return self.end - self.start

default_sim_config = SimConfig(
    bedtime_on_start_day = time(1, 30),
    start_day = datetime.now().date(),
    sleep_duration = timedelta(hours = 8),
    day_length = timedelta(hours = 25),
    stop_date_time = (datetime.now() + timedelta(days=30)),
)

def convert_local_to_utc(dt: datetime) -> datetime:
    dtz_local = dt.astimezone()
    dtz_utc = dtz_local.astimezone(timezone.utc)
    # print(f"Converted {dtz_local} to {dtz_utc}")
    return dtz_utc.replace(tzinfo=None)

def write_to_ics_files(
    sleep_periods: list[Period], awake_periods: list[Period],
    periods_to_include: set[Literal['awake', 'sleep']],
    ics_file: Union[str, Path]
):
    c = Calendar()

    if 'sleep' in periods_to_include:
        for period in sleep_periods:
            e = Event()
            e.name = "Sleep"
            e.begin = convert_local_to_utc(period.start)
            e.end = convert_local_to_utc(period.end)
            c.events.add(e)

    if 'awake' in periods_to_include:
        for period in awake_periods:
            e = Event()
            e.name = "Awake"
            e.begin = convert_local_to_utc(period.start)
            e.end = convert_local_to_utc(period.end)
            c.events.add(e)

    with open(ics_file, 'w') as fp:
        fp.writelines(c.serialize_iter())

def run_sim_print_data(sim_config: SimConfig):
    sleep_periods: list[Period] = []
    awake_periods: list[Period] = []

    # cur_dt is always a sleep start time
    cur_dt: datetime = datetime.combine(sim_config.start_day, sim_config.bedtime_on_start_day)
    print(f"Go to sleep at {cur_dt}")

    last_awake_start: datetime = cur_dt - sim_config.awake_duration
    last_sleep_start: datetime = cur_dt

    while cur_dt <= sim_config.stop_date_time:
        next_awake_start = cur_dt + sim_config.sleep_duration
        print(f"Wake up at {next_awake_start}")

        next_sleep_start = next_awake_start + sim_config.awake_duration
        print(f"Go to sleep at {next_sleep_start}")

        sleep_periods.append(Period(last_sleep_start, next_awake_start))
        awake_periods.append(Period(next_awake_start, next_sleep_start))

        last_awake_start = next_awake_start
        last_sleep_start = next_sleep_start
        cur_dt = next_sleep_start

    print(f"Sleep periods: {sleep_periods}")
    print(f"Awake periods: {awake_periods}")

    write_to_ics_files(sleep_periods, awake_periods, {'sleep'}, '25h_day_schedule.ics')

    print(f"Wrote {len(sleep_periods)} sleep periods and {len(awake_periods)} awake periods to file.")

if __name__ == "__main__":
    run_sim_print_data(default_sim_config)
