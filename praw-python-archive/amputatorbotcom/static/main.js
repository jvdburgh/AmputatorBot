function clickToCopy(buttonId, value){
    const copyTextarea = value;
    copyTextarea.focus();
    copyTextarea.select();

    try {
        const successful = document.execCommand('copy');
        const msg = successful ? 'successful' : 'unsuccessful';
        console.log('Copying text command was ' + msg);
        document.getElementById(buttonId).innerHTML = 'Copied!';
    } catch (err) {
        console.log('Oops, unable to copy');
    }
}

function delete_flash(flash){
    flash.parentElement.remove();
}

function openMenu() {
    document.getElementById("nav-icon3").classList.toggle("open");
    document.getElementById("menu-mobile").classList.toggle("visible");
    document.getElementById("header").classList.toggle("fixed");
}

function toggleOptional() {
    let optionalBody = document.getElementById("optional");
    optionalBody.classList.toggle("d-none");
    if (optionalBody.classList.contains("d-none")){
        document.getElementById("toggle-optional").innerHTML = '+ Show optional settings';
    }
    else {
        document.getElementById("toggle-optional").innerHTML = '- Hide optional settings';
    }

}

function tryExample(){
    document.getElementById("query").value = "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/";
    updateParams();
    submit();
}


window.onload = function runAutoAmputator() {
    let errorAmount;
    let copyButton;

    if (window.location.search.startsWith("?")) {
        copyButton = document.getElementById("copy-button");
        errorAmount = document.getElementsByClassName("alert-danger").length;
        let searchParams = new URLSearchParams(window.location.search);
        if ((!copyButton) && (errorAmount === 0)) {
            let redirectInput = document.getElementById("should_redirect");
            let redirectValue = searchParams.get("r");
            if (redirectValue === "true"){
                if (window.performance){
                    let navEntries = window.performance.getEntriesByType('navigation');
                    if (navEntries.length > 0 && navEntries[0].type === 'back_forward') {
                         redirectInput.value = "false"
                    }
                    else {
                        redirectInput.value = "true";
                    }
                }
                else {
                    redirectInput.value = "true"
                }
            }
            else {
                redirectInput.value = "false"
            }
            document.getElementById("query").value = searchParams.get("q");
            document.getElementById("use_gac").value = searchParams.get("gac");
            document.getElementById("max_depth").value = searchParams.get("md");
            document.getElementById("generate_comment").value = searchParams.get("gc");
            updateParams();
            submit();
        }
    }
};

function updateParams() {
    let searchParams = new URLSearchParams(window.location.search);
    searchParams.set("gc", document.getElementById('generate_comment').value);
    searchParams.set("gac", document.getElementById('use_gac').value);
    searchParams.set("md", document.getElementById('max_depth').value);
    searchParams.set("r", document.getElementById('should_redirect').value);
    searchParams.set("q", document.getElementById('query').value);

    const newRelativePathQuery = window.location.pathname + '?' + searchParams.toString();
    history.pushState(null, '', newRelativePathQuery);
}

function submit() {
    const form = document.getElementById('input_form');
    const submitFormFunction = Object.getPrototypeOf(form).submit;
    submitFormFunction.call(form);
}