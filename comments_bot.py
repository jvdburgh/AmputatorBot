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
					user_agent = "eu.pythoneverywhere.com:AmputatorBot:v1.1 (by /u/Killed_Mufasa)")
	print("Successfully logged in!\n")
	return r

# Main function. Gets last 2000 comments, scans these for AMP links and replies with the direct link
def run_bot(r, comments_replied_to, comments_unable_to_reply):
	print("Obtaining the last 2000 comments in subreddits amputatorbot, audio, bitcoin, chrome, NOT YET conservative, degoogle, europe, google, firefox, gaming, history, movies, programming, robotics, security, seo, tech, technology, test, todayilearned and NOT YET worldnews.\n")

	# Get the latest 2000 comments in select subreddits using Praw.
	for comment in r.subreddit('amputatorbot+audio+chrome+degoogle+europe+google+firefox+gaming+history+movies+programming+robotics+security+seo+tech+technology+test+todayilearned').comments(limit=2000):
		# Resets for every comment
		meets_all_criteria = False
		comment_could_not_reply = False
		comment_could_reply = False
		comments_urls = []
		comments_non_amp_urls = []
		comments_non_amps_urls_amount = 0
		
		# Check: Does the comment contain any AMP links?
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

		# If all conditions are met, start the main part
		if meets_all_criteria:
			try:
				print(comment.body)
				print("Trying to find the submitted url...\n")

				# Scan the comment body for the links
				comments_urls = re.findall("(?P<url>https?://[^\s]+)", comment.body)
				comments_urls_amount = len(comments_urls)

				# Loop through all submitted links	
				for x in range(comments_urls_amount):

					# Isolate the actual URL (remove markdown) (part 1)
					try:
						comments_urls[x] = comments_urls[x].split('](')[-1]
						print("A link: "+ comments_urls[x] +" was stripped of markdown.\n")
					except Exception as e:
						logging.error(traceback.format_exc())
						print("A link couldn't of didn't have to be trimmed.\n")

					# Isolate the actual URL (remove markdown) (part 2)
					if comments_urls[x].endswith(')?'):
						comments_urls[x] = comments_urls[x][:-2]
						print("Trimmed the link with 2 characters.")
					if comments_urls[x].endswith(')'):
						comments_urls[x] = comments_urls[x][:-1]
						print("Trimmed the link with 1 characters.")
					print(comments_urls[x]+"\n")

					# Check: Is the isolated URL really an amp link?
					if "/amp" in comments_urls[x] or ".amp" in comments_urls[x] or "amp." in comments_urls[x] and "https://" in comments_urls[x]:
						print(" [ OK ] The correct Amp link was found: " + comments_urls[x] + "\n")
						print("Retrieving amp page...\n")

						# Premake an urllib request (to fetch the submitted amp page)	
						req = urllib.request.Request(comments_urls[x])
						req.add_header('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:68.0) Gecko/20100101 Firefox/68.0')
						req.add_header('Accept', 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8')
						req.add_header('Accept-Charset', 'ISO-8859-1,utf-8;q=0.7,*;q=0.3')
						req.add_header('Accept-Encoding', 'none')
						req.add_header('Accept-Language', 'en-US,en;q=0.8')
						req.add_header('Connection', 'keep-alive')
						req.add_header('Referer', 'www.reddit.com')

						# Fetch the submitted amp page, if canonical (direct link) was found, save these for later
						try:
							# Fetch submitted amp page
							print("\nNow scanning the Amp link: " + comments_urls[x] + "\n")
							content = urlopen(comments_urls[x])
							
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
									comments_canonical_url = link.get('href')
									print("Found the normal link: "+comments_canonical_url+"\n")

									# If the canonical url is the same as the submitted url, don't use it
									if comments_canonical_url == comments_urls[x]:
										print(" [Error:If] False positive encounterd (comments_canonical_url == comments_urls[x]).")

									# If the canonical url is unique, add the direct link to the array
									else:
										comments_non_amps_urls_amount = len(comments_non_amp_urls)
										comments_canonical_url = "["+str(comments_non_amps_urls_amount+1)+"] **"+comments_canonical_url+"**"
										comments_non_amp_urls.append(comments_canonical_url)
										print(comments_non_amp_urls)

							# If no direct links were found, throw an exception	
							except Exception as e:
								logging.error(traceback.format_exc())
								print(" [ERROR:Exception] The direct link could not be found.")

						# If the submitted page could't be fetched (or something else went wrong), throw an exception
						except Exception as e:
							logging.error(traceback.format_exc())
							print(" [ERROR:Exception] Submitted page could not be fetched or something else")
							
					# If the program fails to get the correct amp link, ignore it.
					else:
						print(" [ERROR:else:] This link is no AMP link: "+comments_urls[x])

			# If the program fails to find any link at all, throw an exception
			except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Looks like something went wrong trying to find the non_amp url.")

			# If no direct links were found, don't reply
			if comments_non_amps_urls_amount == 0:
				print(" [ERROR:If] There were no correct direct links found. There will be no reply made.")

			# If there were direct links found, reply
			else:

				# Try to reply to OP
				try:

					# If there was only one url found, generate a simple comment
					if comments_non_amps_urls_amount == 1:
						comment_reply = "Beep boop, I'm a bot.\n\nIt looks like you shared a Google AMP link. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **"+comments_non_amp_urls[x]+"**.\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( - By )^(Killed_Mufasa)^(, feedback welcome!)\n\n^(Spotted an AMP link in a comment or submission? Mention [u/AmputatorBot](https://www.reddit.com/user/AmputatorBot) in a reply and I'll try to share the direct link.)"

					# If there were multiple urls found, generate a multi-url comment
					if comments_non_amps_urls_amount > 1:
						# Generate string of all found links
						comment_reply_generated = '\n\n'.join(comments_non_amp_urls)
						# Generate entire comment
						comment_reply = "Beep boop, I'm a bot.\n\nIt looks like you shared a couple of Google AMP links. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal pages** instead: \n\n"+comment_reply_generated+"\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( - By )^(Killed_Mufasa)^(, feedback welcome!)\n\n^(Spotted an AMP link in a comment or submission? Mention [u/AmputatorBot](https://www.reddit.com/user/AmputatorBot) in a reply and I'll try to share the direct link.)"

					else:
						print(" [ERROR:Else] There was a weird amount of non_amps_urls found:"+comments_non_amps_urls_amount)

					# Reply to comment
					comment.reply(comment_reply)
					print("Replied to comment #"+comment.id+".\n")
					comment_could_reply = True
					print("Operation succesfull.\n")

				# If the reply didn't got through, throw an exception (can occur when comment gets deleted or when rate limits are exceeded)
				except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Could not reply to post.")
					comment_could_not_reply = True

			# If the reply was successfully send, note this
			if comment_could_reply:
				with open ("comments_replied_to.txt", "a") as f:
					f.write(comment.id + ",")
					comments_replied_to.append(comment.id)
					print("Added the comment id to file: comments_replied_to.txt.\n")
			
			# If the reply could not be made or send, note this
			if comment_could_not_reply:
				with open ("comments_unable_to_reply.txt", "a") as f:
					f.write(comment.id + ",")
					comments_unable_to_reply.append(comment.id)
					print("Added the comment id to file: comments_unable_to_reply.txt.\n")

			# For debugging purposes:
			try:
				print("\ncomments_replied_to.txt was found, the array is now as follows:")
				print(comments_replied_to)
				print("\ncomments_unable_to_reply.txt was found, the array is now as follows:")
				print(comments_unable_to_reply)
				print("\nThe bot has now replied x times:")
				print(len(comments_replied_to))
				print("\nThe bot has now failed to comment x times:")
				print(len(comments_unable_to_reply))
			except:
				logging.error(traceback.format_exc())
				print(" [ERROR:Exception] Something went wrong while printing (those damn printers never work when you need them to!)")

	# Sleep for 90 seconds (to prevent exceeding of rate limits)
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