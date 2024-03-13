const time_element = document.getElementById("time");

if (time_element) {
    const now = new Date();
    const time_string = now.toLocaleTimeString(undefined, {
        hour: "numeric",
        minute: "2-digit",
        hour12: true,
    });
    time_element.innerText = time_string;
    setInterval(() => {
        const now = new Date();
        const time_string = now.toLocaleTimeString(undefined, {
            hour: "numeric",
            minute: "2-digit",
            hour12: true,
        });
        time_element.innerText = time_string;
    }, 5_000);
}
