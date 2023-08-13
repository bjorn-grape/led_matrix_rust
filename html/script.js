function requestToServer (name){
    myip = getIP()
    fetch(myip + "?name=" + name)
        // .then(response => {
        //     // Check if the request was successful
        //     if (!response.ok) {
        //         throw new Error('Network response was not ok');
        //     }
        //     return response.json();  // Parse the response body as JSON
        // })
        // .then(data => {
        //     console.log(data);  // Handle the parsed response data
        // })
        .catch(error => {
            console.log('There was a problem with the fetch operation:', error.message);
        });
}

function adjustLuminosity(direction) {
    // Implement luminosity adjustment code here
    requestToServer("lum_" + direction)
    // console.log('Adjusting luminosity:', direction);
    // console.log('Adjusting luminosity:', getIP());
}

function navigateContent(direction) {
    // Implement content navigation code here
    requestToServer("dir_" + direction)
    // console.log('Navigating content:', direction);
}

function resetContent() {
    // Implement reset functionality here
    requestToServer("reset")

    // console.log('Resetting content.');
}

function togglePlayPause() {
    // Implement play/pause functionality here
    // console.log('Toggling play/pause.');
    requestToServer("toggle_play")
}

