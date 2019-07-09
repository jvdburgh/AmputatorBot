# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa (https://twitter.com/Killed_Mufasa | https://www.reddit.com/user/Killed_Mufasa | https://github.com/KilledMufasa)

# This wonderfull little program is used by u/AmputatorBot (https://www.reddit.com/user/AmputatorBot) to scan comments for AMP links and to reply to OP with the direct link.

# Import a couple of libraries
from bs4 import BeautifulSoup
from urllib.request import urlopen
from datetime import datetime
import urllib.request
import praw
import config
import time
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
					user_agent = "Windows Local Server:AmputatorBot:v1.0 (by /u/Killed_Mufasa)")
	print("Successfully logged in!\n")
	return r

# Main function. Gets last 2000 comments, scans these for AMP links and replies with the direct link
def run_bot(r, comments_replied_to, comments_unable_to_reply):
	print("Obtaining the last 2000 comments in subreddits amputatorbot, audio, bitcoin, chrome, NOT YET conservative, degoogle, europe, gadgets, google, firefox, gaming, history, movies, politicaldiscussion, programming, robotics, security, seo, tech, technology, test, todayilearned and NOT YET worldnews.\n")

	# Get the latest 2000 comments in select subreddits using Praw.
	for comment in r.subreddit('amputatorbot+audio+bitcoin+chrome+degoogle+europe+gadgets+google+firefox+gaming+history+movies+politicaldiscussion+programming+robotics+security+seo+tech+technology+test+todayilearned').comments(limit=2000):
		# Resets for every comment
		meets_all_criteria = False
		could_not_reply = False
		could_reply = False

		# Check: Does the comment contain any amp links?
		if "/amp" in comment.body or ".amp" in comment.body or "amp." in comment.body and "https://" in comment.body:
			print(" [ OK ] #" + comment.id + " contains one or more of the keywords.")

			# Check: Has AmputatorBot tried (and failed) to respond to this comment already?
			if comment.id not in comments_unable_to_reply: 
				print(" [ OK ] #" + comment.id + " hasn't been tried and failed yet.")

				# Check: Has AmputatorBot replied to this comment already?
				if comment.id not in comments_replied_to:
					print(" [ OK ] #" + comment.id + " hasn't been replied to yet.")

					# Check: Is the comment written by u/AmputatorBot?
					if not comment.author == r.user.me():
						meets_all_criteria = True
						print(" [ OK ] #" + comment.id + " isn't posted by me.\n")

					else:
						print(" [ STOP ] #" + comment.id + " is posted by me.\n")
				else:
					print(" [ STOP ] #" + comment.id + " has already been replied to.\n")
			else:
				print(" [ STOP ] #" + comment.id + " has already been tried, but failed.\n")

		# If all criteria are met, try to reply to OP with the direct link
		if meets_all_criteria:
			try:
				print("String with \"/amp\" and \"https://\" found in the comment body of #"+ comment.id+"\n")
				print(comment.body)
				print("Trying to find the submitted url...\n")
				
				# Scan the comment body for the (first) link
				amp_url = re.search("(?P<url>https?://[^\s]+)", comment.body).group("url")

				# Isolate the actual URL (remove markdown) (part 1)
				try:
					amp_url = amp_url.split('](')[-1]
					print("The amp link: "+amp_url+" was stripped of markdown.\n")
				except Exception as e:
					logging.error(traceback.format_exc())
					print("The amp link couldn't of didn't have to be trimmed.\n")

				# Isolate the actual URL (remove markdown) (part 2)
				if amp_url.endswith(')?'):
					amp_url = amp_url[:-2]
					print("Trimmed the amp link with 2 characters.")
				if amp_url.endswith(')'):
					amp_url = amp_url[:-1]
					print("Trimmed the amp link with 1 characters.")
				print(amp_url+"\n")
				
				# Check: Is the isolated URL really an amp link?
				if "/amp" in amp_url or ".amp" in amp_url or "amp." in amp_url and "https://" in amp_url:
					print(" [ OK ] The correct Amp link was found: "+amp_url+"\n")
					print("Retrieving amp page...\n")

					# Premake an urllib request (to fetch the submitted amp page)	
					req = urllib.request.Request(amp_url)
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
						content = urllib.request.urlopen(req)
						print("\nNow scanning the Amp link: " + amp_url + "\n")
						content = urlopen(amp_url)
						
						# Make the received data readable
						print("Making a soup...\n")
						soup = BeautifulSoup(content, features= "lxml")
						print("Making a readable soup...\n")
						soup.prettify()

						# Scan the received data for the direct link
						print("Scanning for all links...\n")
						try:
							# Check for every link on the amp page if it is of the type rel='canonical'
							for link in soup.find_all(rel='canonical'):
								# Get the direct link
								non_amp_url = link.get('href')
								print("Found the normal link: "+non_amp_url+"\n")

							# Generate a comment
							comment_reply = "Beep boop, I'm a bot.\n\nIt looks like you shared a Google AMP link. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **"+non_amp_url+"**.\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( - By )^(Killed_Mufasa)^(, feedback welcome!)"

						# If no direct links were found, throw an exception	
						except Exception as e:
							logging.error(traceback.format_exc())
							print(" [ERROR:Exception] The direct link could not be found.")
							could_not_reply = True

						# Try to reply to OP
						try:
							comment.reply(comment_reply)
							print("Replied to comment #"+comment.id+".\n")
							could_reply = True
							print("Operation succesfull.\n")

						# If the reply didn't got through, throw an exception (can occur when comment gets deleted or when ratelimits are exceeded)
						except Exception as e:
							logging.error(traceback.format_exc())
							print(" [ERROR:Exception] Could not reply to post.")
							could_not_reply = True

					# If the submitted page could't be fetched (or something else went wrong), throw an exception
					except Exception as e:
						logging.error(traceback.format_exc())
						print(" [ERROR:Exception] Submitted page could not be fetched or something else")
						could_not_reply = True

				# If the program fails to get the correct amp link  (which has to be the first link in the comment body (for now)), ignore it.
				else:
					print(" [ERROR:else:] Could not find the correct amp link.")
					could_not_reply = True

			# If the program fails to find any link at all, throw an exception
			except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Looks like something went wrong trying to find the non_amp url.")
					could_not_reply = True
			
			# If the reply was successfully send, note this
			if could_reply:
				comments_replied_to.append(comment.id)
				with open ("comments_replied_to.txt", "a") as f:
					f.write(comment.id + ",")
					print("Added the comment id to file: comments_replied_to.txt.\n")
			
			# If the reply could not be made or send, note this
			if could_not_reply:
				comments_unable_to_reply.append(comment.id)
				with open ("comments_unable_to_reply.txt", "a") as f:
					f.write(comment.id + ",")
					print("Added the comment id to file: comments_unable_to_reply.txt")

			# For debugging purposes:
			try:
				print("Comments_replied_to.txt was found, the array is now as follows:\n"+comments_replied_to)
				print("Comments_unable_to_reply.txt was found, the array is now as follows:\n"+comments_unable_to_reply)
				print(len("The bot has no replied "+comments_replied_to+" times."))
				print(len("The bot failed to comment "+comments_unable_to_reply+" times."))
			except:
				logging.error(traceback.format_exc())
				print(" [ERROR:Exception] Something went wrong while printing (damn printers never work when you need them to!")

	# Sleep for 90 seconds (to prevent exceeding of ratelimits)
	print("\n"+datetime.now().strftime('%Y-%m-%d %H:%M:%S')+": Sleeping for 90 seconds...\n")
	time.sleep(90)

# Get the data of which comments have been replied to
def get_saved_comments():
	if not os.path.isfile("comments_replied_to.txt"):
		comments_replied_to = ['empty']
		print("ERROR: Comments_replied_to.txt could not be found, the array is now as follows:")
		print(comments_replied_to)

	else:
		with open("comments_replied_to.txt", "r") as f:
			comments_replied_to = f.read()
			comments_replied_to = comments_replied_to.split(",")
			print("Comments_replied_to.txt was found, the array is now as follows:")
			print(comments_replied_to)

	return comments_replied_to

# Get the data of which comments could not be replied to (for any reason)
def get_saved_unabletos():
	if not os.path.isfile("comments_unable_to_reply.txt"):
		comments_unable_to_reply = ['empty']
		print("ERROR: Comments_unable_to_reply.txt could not be found, the array is now as follows:")
		print(comments_unable_to_reply)

	else:
		with open("comments_unable_to_reply.txt", "r") as f:
			comments_unable_to_reply = f.read()
			comments_unable_to_reply = comments_unable_to_reply.split(",")
			print("Comments_unable_to_reply.txt could was found, the array is now as follows:")
			print(comments_unable_to_reply)

	return comments_unable_to_reply

# Uses these functions to run the bot
r = bot_login()
comments_replied_to = get_saved_comments()
comments_unable_to_reply = get_saved_unabletos()

# Run the program
while True:
	run_bot(r, comments_replied_to, comments_unable_to_reply)