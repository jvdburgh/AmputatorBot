# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa (https://twitter.com/Killed_Mufasa | https://www.reddit.com/user/Killed_Mufasa | https://github.com/KilledMufasa)

# This wonderfull little program is used by u/AmputatorBot (https://www.reddit.com/user/AmputatorBot) to scan submissions for AMP links and to comment the direct link.

# Import a couple of libraries
from bs4 import BeautifulSoup
from urllib.request import urlopen
import urllib.request
import praw
import config
import os
import re
import traceback
import logging

# Login to Reddit API using Praw. Reads configuration details out of config.py (not public)
def bot_login():
	print("Loggin in...")
	r = praw.Reddit(username = config.username,
					password = config.password,
					client_id = config.client_id,
					client_secret = config.client_secret,
					user_agent = "eu.pythoneverywhere.com:AmputatorBot:v1.1 (by /u/Killed_Mufasa)")
	print("Successfully logged in!\n")
	return r

# Main function. Gets the submissions stream, scans these for AMP links and replies with the direct link
def run_bot(r, comments_replied_to, comments_unable_to_reply):
	print("Obtaining the last 2000 comments in subreddits amputatorbot, audio, bitcoin, chrome, conservative, degoogle, europe, google, firefox, gaming, history, movies, politicaldiscussion, programming, robotics, security, seo, tech, technology, test, todayilearned and NOT YET worldnews.\n")

	# Get the submission stream of select subreddits using Praw.
	for submission in r.subreddit('amputatorbot+audio+chrome+conservative+degoogle+europe+google+firefox+gaming+history+movies+politicaldiscussion+programming+robotics+security+seo+tech+technology+test+todayilearned').stream.submissions():
		# Resets for every submission
		submission_meets_all_criteria = False
		submission_could_not_reply = False
		submission_could_reply = False

		# Check: Does the submitted URL contain any amp links?
		if "/amp" in submission.url or ".amp" in submission.url or "amp." in submission.url and "https://" in submission.url:
			print(" [ OK ] #" + submission.id + " contains one or more of the keywords.")
			
			# Check: Has AmputatorBot tried (and failed) to respond to this submission already?
			if submission.id not in submissions_unable_to_reply: 
				print(" [ OK ] #" + submission.id + " hasn't been tried and failed yet.")

				# Check: Has AmputatorBot replied to this submission already?
				if submission.id not in submissions_replied_to:
					print(" [ OK ] #" + submission.id + " hasn't been replied to yet.")

					# Check: Is the submission posted by u/AmputatorBot?
					if not submission.author == r.user.me():
						submission_meets_all_criteria = True
						print(" [ OK ] #" + submission.id + " isn't posted by me.\n")
						
					else:
						print(" [ STOP ] #" + submission.id + " is posted by me.\n")
				else:
					print(" [ STOP ] #" + submission.id + " has already been replied to.\n")
			else:
				print(" [ STOP ] #" + submission.id + " has already been tried, but failed.\n")

		# If all criteria are met, try to comment with the direct link
		if submission_meets_all_criteria:
			try:
				print("String with \"/amp\" and \"https://\" found in the submitted URL of submission: #"+ submission.id+"\n")
				print("\nSubmission Title: "+submission.title+"\nSubmission ID: "+submission.id+"\nSubmission Body: "+submission.selftext+"\nSubmission URL: "+submission.url)

				# Premake an urllib request (to fetch the submitted amp page)	
				print("\nRetrieving amp page...\n")
				req = urllib.request.Request(submission.url)
				req.add_header('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:68.0) Gecko/20100101 Firefox/68.0')
				req.add_header('Accept', 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8')
				req.add_header('Accept-Charset', 'ISO-8859-1,utf-8;q=0.7,*;q=0.3')
				req.add_header('Accept-Encoding', 'none')
				req.add_header('Accept-Language', 'en-US,en;q=0.8')
				req.add_header('Connection', 'keep-alive')
				req.add_header('Referer', 'www.reddit.com')

				# Fetch the submitted amp page, if canonical (direct link) was found, generate and post comment
				try:
					# Fetch submitted amp page
					submission_content = urllib.request.urlopen(req)
					print("\nNow scanning the submitted Amp link: " + submission.url + "\n")
					submission_content = urlopen(submission.url)

					# Make the received data readable
					print("Making a soup...\n")
					soup = BeautifulSoup(submission_content, features= "lxml")
					print("Making a readable soup...\n")
					soup.prettify()

					# Scan the received data for the direct link
					print("Scanning for the original link...\n")
					try:
						# Check for every link on the amp page if it is of the type rel='canonical'
						for link in soup.find_all(rel='canonical'):
							# Get the direct link
							submission_non_amp_url = link.get('href')
							print("Found the normal link: "+submission_non_amp_url+"\n")

						# Generate a comment	
						submission_reply = "Beep boop, I'm a bot.\n\nIt looks like you shared a Google AMP link. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **"+submission_non_amp_url+"**.\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( - By )^(Killed_Mufasa)^(, feedback welcome!)"
							
					# If no direct links were found, throw an exception	
					except Exception as e:
						logging.error(traceback.format_exc())
						print(" [ERROR:Exception] The direct link could not be found.")
						submission_could_not_reply = True

					# Try to comment on OP's submission with a top-level comment
					try:
						submission.reply(submission_reply)
						print("REPLY="+submission_reply+"\n")
						print("Replied to submission #"+submission.id+".\n")
						submission_could_reply = True
						print("Operation succesfull.\n")
					
					# If the reply didn't got through, throw an exception (can occur when comment gets deleted or when rate limits are exceeded)
					except Exception as e:
						logging.error(traceback.format_exc())
						print(" [ERROR:Exception] Could not reply to submission.")
						submission_could_not_reply = True
						
				# If the submitted page couldn't be fetched (or something else went wrong), throw an exception
				except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Submitted page could not be fetched or something else.")
					submission_could_not_reply = True

			# If something else went wrong, throw an exception
			except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Something went wrong on a weird spot.")
					submission_could_not_reply = True
					
			# If the comment was successfully send, note this
			if submission_could_reply:
				submissions_replied_to.append(submission.id)
				with open ("submissions_replied_to.txt", "a") as f:
					f.write(submission.id + ",")
					print("Added the submission id to the file: submissions_replied_to.txt\n")

			# If the comment could not be made or send, note this
			if submission_could_not_reply:
				submissions_unable_to_reply.append(submission.id)
				with open ("submissions_unable_to_reply.txt", "a") as f:
					f.write(submission.id + ",")
					print("Added the submission id to file: submissions_unable_to_reply.txt\n")

			# For debugging purposes:
			try:
				print("\nSubmissions_replied_to.txt was found, the array is now as follows:")
				print(submissions_replied_to)
				print("\nSubmissions_unable_to_reply.txt was found, the array is now as follows:")
				print(submissions_unable_to_reply)
				print("\nThe bot has now replied x times:")
				print(len(submissions_replied_to))
				print("\nThe bot has now failed to comment x times:")
				print(len(submissions_unable_to_reply))
			except:
				logging.error(traceback.format_exc())
				print(" [ERROR:Exception] Something went wrong while printing (those damn printers never work when you need them to!)")

# Get the data of which submissions have been replied to
def get_saved_submissions_repliedtos():
	if not os.path.isfile("submissions_replied_to.txt"):
		submissions_replied_to = ['empty']
		print("ERROR: Submissions_replied_to.txt could not be found, the array is now as follows:")
		print(submissions_replied_to)

	else:
		with open("submissions_replied_to.txt", "r") as f:
			submissions_replied_to = f.read()
			submissions_replied_to = submissions_replied_to.split(",")
			print("Submissions_replied_to.txt was found, the array is now as follows:")
			print(submissions_replied_to)

	return submissions_replied_to

# Get the data of which submissions could not be replied to (for any reason)
def get_saved_submissions_unabletos():
	if not os.path.isfile("submissions_unable_to_reply.txt"):
		submissions_unable_to_reply = ['empty']
		print("ERROR: Submissions_unable_to_reply.txt could not be found, the array is now as follows:")
		print(submissions_unable_to_reply)

	else:
		with open("submissions_unable_to_reply.txt", "r") as f:
			submissions_unable_to_reply = f.read()
			submissions_unable_to_reply = submissions_unable_to_reply.split(",")
			print("Submissions_unable_to_reply.txt was found, the array is now as follows:")
			print(submissions_unable_to_reply)

	return submissions_unable_to_reply

# Uses these functions to run the bot
r = bot_login()
submissions_replied_to = get_saved_submissions_repliedtos()
submissions_unable_to_reply = get_saved_submissions_unabletos()

# Run the program
while True:
	run_bot(r, submissions_replied_to, submissions_unable_to_reply)