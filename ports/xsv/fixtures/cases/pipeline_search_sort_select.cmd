xsv search -s city 'Oslo|Rome' people.csv | xsv sort -s name | xsv select name,city
