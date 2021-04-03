import matplotlib.pyplot as plt
import matplotlib 


duration_map = dict()


def handle_request(status_code, body, duration, connections):
    if status_code != 200: # Lets only track successful requests
        return
    global duration_map
    if connections in  duration_map:
        duration_map[connections].append(duration)
    else:
        duration_map[connections] = [duration]


def teardown():
    global duration_map

    matplotlib.use("Agg")
    fig, ax = plt.subplots()

    keys = sorted(duration_map.keys())

    data = []
    for k in keys:
        data.append(duration_map[k])

    ax.set_title('Request duration violin plots')
    ax.set_ylabel('Durations')
    ax.violinplot(data)

    ax.set_xticks(range(1, len(keys)+1))
    ax.set_xticklabels(keys)
    ax.set_xlabel('Number of concurrent requests')

    plt.savefig('req_duration_violinplot.png')

