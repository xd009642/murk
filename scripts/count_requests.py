import matplotlib.pyplot as plt
import matplotlib 


duration_map = dict()


def handle_request(status_code, body, duration, connections):
    if status_code != 200: # Lets only track successful requests
        return
    global duration_map
    if connections in  duration_map:
        duration_map[connections] += 1
    else:
        duration_map[connections] = 1


def teardown():
    global duration_map

    matplotlib.use("Agg")
    fig, ax = plt.subplots()

    keys = sorted(duration_map.keys())

    data = []
    for k in keys:
        data.append(duration_map[k])

    ax.set_title('Number of completed requests')
    ax.set_ylabel('Number of requests')
    ax.plot(keys, data)

    ax.set_xticks(range(1, len(keys)+1))
    ax.set_xticklabels(keys)
    ax.set_xlabel('Number of concurrent requests')

    plt.savefig("request_count.png");
