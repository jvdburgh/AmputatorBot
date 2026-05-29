import distutils.util
import re
import sys
import traceback
from datetime import datetime
from typing import Tuple, Union

import jsons
from flask import Flask, request, redirect, render_template, flash, Markup, jsonify, Response

from datahandlers.local_datahandler import get_data_by_filename as get_data
from datahandlers.remote_datahandler import get_engine_session, add_data, save_entry
from forms import InputForm
from helpers import logger
from helpers.criteria_checker import check_criteria
from helpers.flashmessage_generator import generate_flashmessage, generate_simplified_comment
from helpers.utils import get_urls, get_urls_info
from models.item import Item
from models.resultcode import ResultCode
from models.type import Type
from static import static

app = Flask(__name__)
app.config['SECRET_KEY'] = 'REDACTED'

log = logger.get_log(sys)
problematic_domains = get_data("problematic_domains")


@app.route("/", methods=['GET', 'POST'], endpoint='n')
@app.route("/amputatorbot", methods=['GET', 'POST'], endpoint='old')
def run_amputatorbotcom(type=Type.ONLINE, save_to_database=True) -> Union[Response, str]:
    form = InputForm()

    if form.validate_on_submit():

        query = get_query_string_value(request, "q")
        use_gac = get_query_bool_value(request, "gac", True)
        max_depth = get_query_int_value(request, "md", static.MAX_DEPTH)
        generate_comment = get_query_bool_value(request, "gc", False)
        should_redirect = get_query_bool_value(request, "r", False)

        i = Item(
            type=type,
            body=query
        )

        log.info(f"\nNew entry: {i.body} on {request.endpoint} with gac {use_gac} "
                 f"& md {max_depth} & gc {generate_comment} & r {should_redirect} ")

        meets_criteria, result_code = check_criteria(
            item=i,
            mustBeAMP=True
        )

        if meets_criteria:
            log.info(f"Entry meets criteria, resultcode = {result_code}")

            # Get the urls from the body and try to find the canonicals
            urls = get_urls(i.body)
            i.links = get_urls_info(urls, use_gac, max_depth)

            # If a canonical was found, generate a reply, otherwise log a warning
            if any(link.canonical for link in i.links) or any(link.amp_canonical for link in i.links):
                result_code = ResultCode.SUCCESS

                if should_redirect:
                    save_entry(save_to_database=save_to_database, entry_type=type.value, links=i.links)

                    if i.links[0].canonical.url:
                        log.info(f"Redirecting to {i.links[0].canonical.url}")
                        return redirect(i.links[0].canonical.url, 303)
                    if i.links[0].amp_canonical:
                        log.info(f"Redirecting to {i.links[0].amp_canonical.url}")
                        return redirect(i.links[0].amp_canonical.url, 303)

                message = generate_flashmessage(result_code=result_code, links=i.links)
                message = Markup(message)
                flash(message, "success")

                if generate_comment:
                    message2 = generate_simplified_comment(links=i.links)
                    message2 = Markup(message2)
                    flash(message2, "success")

            else:
                log.warning("No canonicals found")
                if any(link.origin.domain in problematic_domains for link in i.links):
                    log.info("Problematic domain detected")
                    result_code = ResultCode.ERROR_PROBLEMATIC_DOMAIN
                else:
                    result_code = ResultCode.ERROR_NO_CANONICALS
                show_danger_result_code_flash(result_code)

                show_info_flash()

            save_entry(save_to_database=save_to_database, entry_type=type.value, links=i.links)

        elif result_code == ResultCode.ERROR_NO_AMP:
            show_danger_result_code_flash(result_code)
            show_info_flash()

    else:
        show_info_flash()

    return render_template('form.html', title='form', form=form)


def get_query_string_value(request, arg) -> str:
    if arg in request.args and request.args[arg] != "":
        return str(request.args[arg])
    else:
        return ""


def get_query_url(request) -> str:
    raw_query_string = str(request.query_string.decode("UTF-8"))

    if raw_query_string != "":
        query = re.sub("&md=\\w", "", raw_query_string)
        query = re.sub("md=\\w&", "", query)
        query = re.sub("&gac=(?:true|false)", "", query)
        query = re.sub("gac=(?:true|false)&", "", query)
        query = re.sub("q=", "", query)
        return query
    else:
        return ""


def get_query_bool_value(request, arg, defaultValue) -> bool:
    if arg in request.args and request.args[arg] != "":
        return bool(distutils.util.strtobool(request.args[arg]))
    else:
        return defaultValue


def get_query_int_value(request, arg, defaultValue) -> int:
    if arg in request.args and request.args[arg] != "":
        return int(request.args[arg])
    else:
        return defaultValue


def show_info_flash():
    message = Markup("<div class='flash-intro'><span class='desktopOnly'>Most AMP pages can be easily "
                     "recognized by strings like 'amp' in their URLs, for example: "
                     "<b>google.com/amp</b>/.. or bbc.com/news/<b>amp</b>/.. </span>No AMP link at hand, but "
                     "just curious?<input class='try-example desktopOnly' id='try-example' onclick='tryExample()' "
                     "name='try-example' type='submit' value='Try with an example of a valid AMP URL'>"
                     "<input class='try-example mobileOnly' id='try-example' onclick='tryExample()' "
                     "name='try-example' type='submit' value='Try with an example'></div>")
    flash(message, "info")


def show_danger_result_code_flash(result_code):
    message = generate_flashmessage(result_code=result_code)
    message = Markup(message)
    flash(message, "danger")


@app.route("/api/v1/convert", methods=['GET', 'POST'])
def run_api(type=Type.API, authorization_required=False, save_to_database=True) -> Tuple[Response, int]:
    try:
        if authorization_required:
            # Test authentication of user
            api_key = request.headers.get('authorization')
            if not api_key:
                return jsonify({
                    "error_message": "Error: Authentication failed: Missing authorization header. Make sure to "
                                     "include a valid AmputatorBot API key in your request. Feel free to contact "
                                     "u/Killed_Mufasa on Reddit if you would like such a key.",
                    "result_code": ResultCode.API_ERROR_NO_AUTHORIZATION.value
                }), 401

            # Get the token (strips out 'Bearer ')
            api_key = api_key.partition(" ")[2]

            # Check if the token is authorized
            is_authorized = static.API_KEYS.get(api_key)
            if not is_authorized:
                log.warning(f"Authentication failed, provided key: {api_key}")
                return jsonify({
                    "error_message": "Error: Authentication failed: This api_key doesn't exist. Make sure to include a "
                                     "valid AmputatorBot API key in your request. Feel free to contact u/Killed_Mufasa "
                                     "on Reddit if you would like such a key.",
                    "result_code": ResultCode.API_ERROR_AUTHENTICATION_FAILED.value
                }), 401

            else:
                log.info(f"Successful authentication by {is_authorized}")

        # Get the query manually. But, if the query url has %20 in it, it's probably text.
        # If so, get the query using args instead. This allows us to find more links correctly,
        # while reducing the risk of badly parsed URLs. Bit dirty, but folks should probably use
        # post instead anyway.
        if "q" in request.args:
            query = get_query_url(request)
            if query.__contains__("%20"):
                query = get_query_string_value(request, "q")

        else:
            return jsonify({
                "error_message": "Error: No query field provided. Please specify a query (q=)",
                "result_code": ResultCode.API_ERROR_REQUIRED_FIELD_MISSING.value
            }), 400

        # Generate an item object, check the criteria
        i = Item(
            type=type,
            body=query
        )

        use_gac = get_query_bool_value(request, "gac", True)
        max_depth = get_query_int_value(request, "md", static.MAX_DEPTH)

        log.info(f"\nNew {type.name} entry on {request.endpoint}, q = {query}, gac = {use_gac}, "
                 f"md = {max_depth}")

        meets_criteria, result_code = check_criteria(
            item=i,
            mustBeAMP=True
        )

        if meets_criteria:
            log.info(f"Entry meets criteria, resultcode = {result_code}")

            # Get the urls from the body and try to find the canonicals
            urls = get_urls(i.body)
            i.links = get_urls_info(urls, use_gac, max_depth)

            # Dump it into a json
            urls_info_json = jsons.dump(i.links)

            save_entry(save_to_database=save_to_database, entry_type=type.value, links=i.links)

            # If a canonical was found, return the json, otherwise return a warning
            if any(link.canonical for link in i.links) or any(link.amp_canonical for link in i.links):
                return jsonify(urls_info_json), 200

            else:
                log.warning("No canonicals found")
                if any(link.origin.domain in problematic_domains for link in i.links):
                    log.info("Problematic domain detected")
                    result_code = ResultCode.ERROR_PROBLEMATIC_DOMAIN
                    return jsonify({
                        "error_message": "Error: No canonicals found, this domain is known to be problematic",
                        "result_code": result_code.value
                    }), 561

                else:
                    result_code = ResultCode.ERROR_NO_CANONICALS
                    return jsonify({
                        "error_message": "Error: No canonicals found",
                        "result_code": result_code.value
                    }), 560

        else:
            return jsonify({
                "error_message": "Error: Entry doesn't meet criteria (no AMP link detected)",
                "result_code": result_code.value
            }), 406

    except (ValueError, Exception):
        log.error(traceback.format_exc())
        return jsonify({
            "error_message": "Error: An unknown error was raised",
            "result_code": ResultCode.ERROR_UNKNOWN.value
        }), 500


"""
@app.errorhandler(InternalServerError)
def handle_500(e):
    original = getattr(e, "original_exception", None)

    if original is None:
        # direct 500 error, such as abort(500)
        return render_template("500.html"), 500

    # wrapped unhandled error
    return render_template("500_unhandled.html", e=original), 500
"""

if __name__ == '__main__':
    app.run(debug=False)
