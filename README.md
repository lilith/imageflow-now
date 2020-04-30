# Basic S3 Image Processing Proxy

To configure, go to your Vercel dashboard, project settings, and add env vars `IMAGEFLOW_AWS_ACCESS_KEY_ID` and `IMAGEFLOW_AWS_ACCESS_KEY_SECRET` with your AWS access key ID and secret respectively. 

/api/imageflow/[region]/[bucket]/path/to/image.jpg?width=200

Uses the same [querystring commands](https://imageresizing.net/docs/v4/docs/basics) as ImageResizer 4, plus a few more. 
