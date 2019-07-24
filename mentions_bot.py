# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa (https://twitter.com/Killed_Mufasa | https://www.reddit.com/user/Killed_Mufasa | https://github.com/KilledMufasa)

# This wonderfull little program is used by u/AmputatorBot (https://www.reddit.com/user/AmputatorBot) to scan u/AmputatorBot's inbox for mentions, and to reply to the correct comment.

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

# Main function. Gets the inbox stream, filters for mentions, scans the context for AMP links and replies with the direct link (https://praw.readthedocs.io/en/latest/code_overview/reddit/inbox.html)
def run_bot(r, mentions_replied_to, mentions_unable_to_reply, forbidden_subreddits):
	print("Obtaining the stream of inbox items as they become available.")
	
	# Get the inbox stream using Praw (initially returns 100 historical items).
	for mention in praw.models.util.stream_generator(r.inbox.mentions):

		# Resets for every mention
		reply_to_parent = False
		mention_could_not_reply = False
		mention_could_reply = False
		parent_is_comment = False
		parent_is_submission = False
		mentions_urls = []
		mentions_non_amp_urls = []
		mentions_non_amps_urls_amount = 0
		mentions_canonical_url = ""
		
		# Print a notice that u/AmputatorBot has been mentioned
		parent = mention.parent()
		print("\n\nu/AmputatorBot has been mentioned. Mention comment ID: " + mention.id + "\nParent comment ID: "+ parent.id)

		# Firguring out if parent is a submission or comment
		print("Checking if parent is a comment or submission...")
		try:
			# Check if the parent is a comment, if yes get comment body
			if(isinstance(parent, praw.models.Comment)):
				print(" [ OK ] The parent #" + parent.id + " is a comment.")
				parent_body = parent.body
				parent_is_comment = True

			# Check if the parent is a submission, if yes get submission body
			if(isinstance(parent, praw.models.Submission)):
				print(" [ OK ] The parent #" + parent.id + " is a submission.")
				parent_body = parent.url
				parent_is_submission = True

		except:
			logging.error(traceback.format_exc())
			print(" [ERROR:Exception] There was a weird instance found.")
			mention_could_not_reply = True

		# Check if the parent comment contains an AMP link
		if "/amp" in parent_body or ".amp" in parent_body or "amp." in parent_body or "?amp" in parent_body or "amp?" in parent_body or "=amp" in parent_body or "amp=" in parent_body and "https://" in parent_body:
			print(" [ OK ] #" + parent.id + " (the parent comment) contains one or more of the keywords.")

			# Check: Is AmputatorBot allowed in called subreddit?
			if mention.subreddit.display_name not in forbidden_subreddits:
				print(" [ OK ] #" + parent.id + " called in a subreddit were the bot is not forbidden.")

				# Check: Has AmputatorBot tried (and failed) to respond to this mention already?
				if parent.id not in mentions_unable_to_reply: 
					print(" [ OK ] #" + parent.id + " hasn't been tried and failed yet.")

					# Check: Has AmputatorBot replied to this mention already?
					if parent.id not in mentions_replied_to:
						print(" [ OK ] #" + parent.id + " hasn't been replied to yet.")

						# Check: Is the mention posted by u/AmputatorBot?
						if not parent.author == r.user.me():
							reply_to_parent = True
							print(" [ OK ] #" + parent.id + " isn't posted by me.\n")
							
						else:
							print(" [ STOP ] #" + parent.id + " is posted by me.\n")

					else:
						print(" [ STOP ] #" + parent.id + " has already been replied to.\n")
				
				else:
					print(" [ STOP ] #" + parent.id + " has already been tried, but failed.\n")
				
			else:
				print(" [ STOP ] #" + parent.id + " was called in a subreddit were bot is forbidden.\n")

		else:
			print(" [ STOP ] #" + parent.id + " parent did not contain an amp link.\n")

		# If the parent is a comment, try to reply with the direct link
		if reply_to_parent:
			try:
				print(parent_body)
				print("Trying to find the submitted url...\n")

				# Scan the comment body for the links
				mentions_urls = re.findall("(?P<url>https?://[^\s]+)", parent_body)
				mentions_urls_amount = len(mentions_urls)

				# Loop through all submitted links	
				for x in range(mentions_urls_amount):

					# Isolate the actual URL (remove markdown) (part 1)
					try:
						mentions_urls[x] = mentions_urls[x].split('](')[-1]
						print("A link: "+ mentions_urls[x] +" was stripped of markdown.\n")
					except Exception as e:
						logging.error(traceback.format_exc())
						print("A link couldn't of didn't have to be trimmed.\n")

					# Isolate the actual URL (remove markdown) (part 2)
					if mentions_urls[x].endswith(')?'):
						mentions_urls[x] = mentions_urls[x][:-2]
						print("Trimmed the link with 2 characters.")
					if mentions_urls[x].endswith(')'):
						mentions_urls[x] = mentions_urls[x][:-1]
						print("Trimmed the link with 1 character.")
					print(mentions_urls[x]+"\n")

					# Check: Is the isolated URL really an amp link?
					if "/amp" in mentions_urls[x] or ".amp" in mentions_urls[x] or "amp." in mentions_urls[x] or "?amp" in mentions_urls[x] or "amp" in mentions_urls[x] and "https://" in mentions_urls[x]:
						print(" [ OK ] The correct Amp link was found: " + mentions_urls[x] + "\n")
						print("Retrieving amp page...\n")

						# Premake an urllib request (to fetch the submitted amp page)	
						req = urllib.request.Request(mentions_urls[x])
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
							print("\nNow scanning the Amp link: " + mentions_urls[x] + "\n")
							content = urlopen(mentions_urls[x])
							
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
									mentions_canonical_url = link.get('href')
									print("Found the normal link: "+mentions_canonical_url+"\n")

									# If the canonical url is the same as the submitted url, don't use it
									if mentions_canonical_url == mentions_urls[x]:
										print(" [Error:If] False positive encounterd (mentions_canonical_url == mentions_urls[x]).")
										mention_could_not_reply = True

									# If the canonical url is unique, add the direct link to the array
									else:
										mentions_non_amps_urls_amount = len(mentions_non_amp_urls)
										mentions_canonical_url_markdown = "["+str(mentions_non_amps_urls_amount+1)+"] **"+mentions_canonical_url+"**"
										mentions_non_amp_urls.append(mentions_canonical_url_markdown)
										print(mentions_non_amp_urls)

							# If no direct links were found, throw an exception	
							except Exception as e:
								logging.error(traceback.format_exc())
								print(" [ERROR:Exception] The direct link could not be found.")
								mention_could_not_reply = True

						# If the submitted page could't be fetched (or something else went wrong), throw an exception
						except Exception as e:
							logging.error(traceback.format_exc())
							print(" [ERROR:Exception] Submitted page could not be fetched or something else")
							mention_could_not_reply = True
							
					# If the program fails to get the correct amp link, ignore it.
					else:
						print(" [ERROR:else:] This link is no AMP link: "+mentions_urls[x])

			# If the program fails to find any link at all, throw an exception
			except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Looks like something went wrong trying to find the non_amp url.")
					mention_could_not_reply = True

			# If no direct links were found, don't reply
			mentions_non_amps_urls_amount = len(mentions_non_amp_urls)
			print("The array of submitted urls")
			print(mentions_non_amp_urls)
			print("The amount of non amp urls found")
			print(mentions_non_amps_urls_amount)
			if mentions_non_amps_urls_amount == 0:
				print(" [ERROR:If] There were no correct direct links found. There will be no reply made.")
				submission_could_not_reply = True

			# If there were direct links found, reply
			else:

				# Try to reply to OP
				try:

					# If there was only one url found, generate a simple comment
					if mentions_non_amps_urls_amount == 1:
						mention_reply = "Beep boop, I'm a bot. It looks like someone shared a Google AMP link. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **"+mentions_canonical_url+"**.\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( | )[^(Mention to summon)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"
					# If there were multiple urls found, generate a multi-url comment
					if mentions_non_amps_urls_amount > 1:
						# Generate string of all found links
						mention_reply_generated = '\n\n'.join(mentions_non_amp_urls)
						# Generate entire comment
						mention_reply = "Beep boop, I'm a bot. It looks like someone shared a couple of Google AMP links. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal pages** instead: \n\n"+mention_reply_generated+"\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( | )[^(Mention to summon)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

					# Reply to mention
					mention.reply(mention_reply)
					print("Replied to comment #"+parent.id+".\n")
					mention_could_reply = True
					print("Operation succesfull.\n")

				# If the reply didn't got through, throw an exception (can occur when comment gets deleted or when rate limits are exceeded)
				except Exception as e:
					logging.error(traceback.format_exc())
					print(" [ERROR:Exception] Could not reply to post.")
					mention_could_not_reply = True

			# If the reply was successfully send, note this
			if mention_could_reply:
				with open ("mentions_replied_to.txt", "a") as f:
					f.write(parent.id + ",")
					mentions_replied_to.append(parent.id)
					print("Added the mention id to file: mentions_replied_to.txt.\n")
			
			# If the reply could not be made or send, note this
			if mention_could_not_reply:
				with open ("mentions_unable_to_reply.txt", "a") as f:
					f.write(parent.id + ",")
					mentions_unable_to_reply.append(parent.id)
					print("Added the mention id to file: mentions_unable_to_reply.txt.\n")

			# For debugging purposes:
			'''
			try:
				print("\nmentions_replied_to.txt was found, the array is now as follows:")
				print(mentions_replied_to)
				print("\nmentions_unable_to_reply.txt was found, the array is now as follows:")
				print(mentions_unable_to_reply)
				print("\nThe bot has now replied x times:")
				print(len(mentions_replied_to))
				print("\nThe bot has now failed to comment x times:")
				print(len(mentions_unable_to_reply))
			except:
				logging.error(traceback.format_exc())
				print(" [ERROR:Exception] Something went wrong while printing (those damn printers never work when you need them to!)")
			'''

# Get the data of which mentions have been replied to
def get_saved_mentions_repliedtos():
	if not os.path.isfile("mentions_replied_to.txt"):
		mentions_replied_to = ['empty']
		print("ERROR: mentions_replied_to.txt could not be found, the array is now as follows:")
		print(mentions_replied_to)

	else:
		with open("mentions_replied_to.txt", "r") as f:
			mentions_replied_to = f.read()
			mentions_replied_to = mentions_replied_to.split(",")
			print("mentions_replied_to.txt was found, the array is now as follows:")
			print(mentions_replied_to)

	return mentions_replied_to

# Get the data of which mentions could not be replied to (for any reason)
def get_saved_mentions_unabletos():
	if not os.path.isfile("mentions_unable_to_reply.txt"):
		mentions_unable_to_reply = ['empty']
		print("ERROR: mentions_unable_to_reply.txt could not be found, the array is now as follows:")
		print(mentions_unable_to_reply)

	else:
		with open("mentions_unable_to_reply.txt", "r") as f:
			mentions_unable_to_reply = f.read()
			mentions_unable_to_reply = mentions_unable_to_reply.split(",")
			print("mentions_unable_to_reply.txt was found, the array is now as follows:")
			print(mentions_unable_to_reply)

	return mentions_unable_to_reply

# Get the data of which subreddits the bot is forbidden to post in
def get_forbidden_subreddits():
	if not os.path.isfile("forbidden_subreddits.txt"):
		forbidden_subreddits = ['empty']
		print("ERROR: forbidden_subreddits.txt could not be found, the array is now as follows:")
		print(forbidden_subreddits)

	else:
		with open("forbidden_subreddits.txt", "r") as f:
			forbidden_subreddits = f.read()
			forbidden_subreddits = forbidden_subreddits.split(",")
			print("forbidden_subreddits.txt was found, the array is now as follows:")
			print(forbidden_subreddits)

	return forbidden_subreddits


# Uses these functions to run the bot
r = bot_login()
mentions_replied_to = get_saved_mentions_repliedtos()
mentions_unable_to_reply = get_saved_mentions_unabletos()
forbidden_subreddits = get_forbidden_subreddits()


# Run the program
while True:
	run_bot(r, mentions_replied_to, mentions_unable_to_reply, forbidden_subreddits)